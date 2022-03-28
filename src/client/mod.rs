use crate::{Format, Read, Type};
use random::Source;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::num::NonZeroU8;
use std::str::FromStr;
use std::time::Duration;

#[derive(Debug, Clone, derive_builder::Builder)]
pub struct ConnectionOptions {
    #[builder(setter(into, strip_option), default)]
    local_ip: Option<IpAddr>,
    #[builder(setter(into, strip_option), default)]
    local_port: Option<u16>,
    #[builder(setter(into))]
    remote_ip: IpAddr,
    #[builder(setter(into), default = "51")]
    remote_port: u16,
    #[builder(default = "Duration::from_secs(2)")]
    timeout: Duration,
    #[builder(default = "NonZeroU8::new(3).unwrap()")]
    resend_attempts: NonZeroU8,
    #[builder(default = "1024")]
    rx_buffer_size: usize,
}

impl ConnectionOptionsBuilder {
    pub fn remote_host(mut self, host: &str) -> Self {
        use std::net::ToSocketAddrs;
        self.remote_ip = (host, 0)
            .to_socket_addrs()
            .ok()
            .and_then(|mut i| i.next())
            .map(|s| s.ip());
        self
    }

    pub async fn remote_host_async(mut self, host: &str) -> Self {
        self.remote_ip = match host.parse::<IpAddr>() {
            Ok(ip) => Some(ip),
            Err(_) => tokio::net::lookup_host(host)
                .await
                .ok()
                .and_then(|mut i| i.next())
                .map(|s| s.ip()),
        };

        self
    }
}

impl ConnectionOptions {
    pub fn new_onewire_read<'a>(
        &self,
        devices: impl Iterator<Item = &'a onewire::Device>,
    ) -> Result<Request, crate::Error> {
        let request = crate::Request::ReadSpecified(random::default().read(), crate::Bus::OneWire);

        let serialized = {
            let mut binary = Vec::new();
            request.write(&mut binary)?;
            binary.extend(devices.flat_map(|d| d.address.iter().cloned()));
            binary
        };

        Ok(Request {
            connection_options: self.clone(),
            request,
            serialized,
        })
    }

    pub fn local_address(&self) -> SocketAddr {
        SocketAddr::new(
            self.local_ip.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
            self.local_port.unwrap_or(0),
        )
    }

    pub fn remote_address(&self) -> SocketAddr {
        SocketAddr::new(self.remote_ip, self.remote_port)
    }
}

#[derive(Debug)]
pub struct Request {
    connection_options: ConnectionOptions,
    request: crate::Request,
    serialized: Vec<u8>,
}

impl Request {
    pub fn dispatch(self) -> Result<Response, DispatchError> {
        tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .unwrap()
            .block_on(self.dispatch_async())
    }

    pub async fn dispatch_async(self) -> Result<Response, DispatchError> {
        let mut buffer = vec![0u8; self.connection_options.rx_buffer_size];
        let socket =
            match tokio::net::UdpSocket::bind(self.connection_options.local_address()).await {
                Ok(socket) => socket,
                Err(source) => {
                    return Err(DispatchError::Io {
                        request: self,
                        source,
                    })
                }
            };

        for send_counter in 0..self.connection_options.resend_attempts.get() {
            if let Err(source) = socket
                .send_to(
                    &self.serialized[..],
                    self.connection_options.remote_address(),
                )
                .await
            {
                return Err(DispatchError::Io {
                    request: self,
                    source,
                });
            }

            match tokio::time::timeout(
                self.connection_options.timeout,
                socket.recv_from(&mut buffer),
            )
            .await
            {
                Ok(Ok((len, from))) => {
                    if from == self.connection_options.remote_address() {
                        let (response, payload_size) = {
                            let mut reader = &buffer[..len];
                            match crate::Response::read(&mut reader) {
                                Ok(response) => (response, reader.available()),
                                Err(source) => {
                                    return Err(DispatchError::ProtocolError {
                                        request: self,
                                        source,
                                    })
                                }
                            }
                        };

                        return Ok(Response {
                            request: self.request,
                            response,
                            payload: buffer
                                .into_iter()
                                .skip(len - payload_size)
                                .take(payload_size)
                                .collect::<Vec<u8>>(),
                            requests_sent: send_counter.saturating_add(1),
                        });
                    } else {
                        eprintln!(
                            "Received response of len={} from unexpected source: {:?}",
                            len, from
                        )
                    }
                }
                Ok(Err(source)) => {
                    return Err(DispatchError::Io {
                        request: self,
                        source,
                    });
                }
                // timeout, retry
                Err(_) => {}
            }
        }
        Err(DispatchError::Timeout)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    #[error("Input/Output Error {source}")]
    Io {
        request: Request,
        #[source]
        source: std::io::Error,
    },
    #[error("All requests remained unanswered")]
    Timeout,
    #[error("An error occurred on the underlying protocol {source}")]
    ProtocolError {
        request: Request,
        #[source]
        source: crate::Error,
    },
}

#[derive(Debug)]
pub struct Response {
    request: crate::Request,
    response: crate::Response,
    payload: Vec<u8>,
    requests_sent: u8,
}

impl Response {
    pub fn extract_values_f32(&self) -> Option<Vec<f32>> {
        match &self.response {
            crate::Response::Ok(_, Format::ValueOnly(Type::F32)) => Some(
                self.payload
                    .chunks_exact(0_f32.to_be_bytes().len())
                    .map(|chunk| f32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect(),
            ),
            crate::Response::Ok(_, Format::AddressValuePairs(Type::Bytes(addr_len), Type::F32)) => {
                Some(
                    self.payload
                        .chunks_exact(usize::from(*addr_len) + 0_f32.to_be_bytes().len())
                        .map(|chunk| {
                            f32::from_be_bytes([
                                chunk[usize::from(*addr_len) + 0],
                                chunk[usize::from(*addr_len) + 1],
                                chunk[usize::from(*addr_len) + 2],
                                chunk[usize::from(*addr_len) + 3],
                            ])
                        })
                        .collect(),
                )
            }
            _ => None,
        }
    }
}

#[cfg(test)]
#[cfg_attr(test, test)]
pub fn sample_usage() {
    let options = ConnectionOptionsBuilder::default()
        .remote_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 5, 112)))
        .remote_port(51_u16)
        .build()
        .unwrap();

    let request = options
        .new_onewire_read(&[
            onewire::Device::from_str("28:ff:f3:54:c1:17:05:33").unwrap(),
            onewire::Device::from_str("28:ff:fe:35:c1:17:05:c0").unwrap(),
        ])
        .unwrap();

    let response = request.dispatch().unwrap();

    println!("{:?}", response.response);
    println!("{:?}", response);

    assert!(matches!(response.response, crate::Response::Ok(_, _)))
}
