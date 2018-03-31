#![no_std]

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Error {
    BufferToSmall,
    UnexpectedEOF,
    UnknownTypeIdentifier,
}


#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Request {
    ReadSpecified(u8, Bus),
    ReadAll(u8),
    ReadAllOnBus(u8, Bus),
    DiscoverAll(u8),
    DiscoverAllOnBus(u8, Bus),

    RetrieveNetworkConfiguration(u8),
    RetrieveVersionInformation(u8),

    SetNetworkMac(u8, [u8; 6]),
    SetNetworkIpSubnetGateway(u8, [u8; 4], [u8; 4], [u8; 4]),
}

impl Request {
    pub fn id(&self) -> u8 {
        match self {
            &Request::ReadSpecified(id, _) => id,
            &Request::ReadAll(id) => id,
            &Request::ReadAllOnBus(id, _) => id,
            &Request::DiscoverAll(id) => id,
            &Request::DiscoverAllOnBus(id, _) => id,
            &Request::RetrieveNetworkConfiguration(id) => id,
            &Request::RetrieveVersionInformation(id) => id,
            &Request::SetNetworkMac(id, _) => id,
            &Request::SetNetworkIpSubnetGateway(id, _, _, _) => id,
        }
    }

    pub fn write(&self, writer: &mut Write) -> Result<usize, Error> {
        Ok(match *self {
            Request::ReadSpecified(id, bus) => {
                writer.write_u8(0x00)?
                    + writer.write_u8(id)?
                    + bus.write(writer)?
            },
            Request::ReadAll(id) => {
                writer.write_u8(0x01)?
                    + writer.write_u8(id)?
            },
            Request::ReadAllOnBus(id, bus) => {
                writer.write_u8(0x02)?
                    + writer.write_u8(id)?
                    + bus.write(writer)?
            },
            Request::DiscoverAll(id) => {
                writer.write_u8(0x10)?
                    + writer.write_u8(id)?
            },
            Request::DiscoverAllOnBus(id, bus) => {
                writer.write_u8(0x11)?
                    + writer.write_u8(id)?
                    + bus.write(writer)?
            },


            Request::SetNetworkMac(id, mac) => {
                writer.write_u8(0xA0)?
                    + writer.write_u8(id)?
                    + writer.write_all(&mac)?
            },
            Request::SetNetworkIpSubnetGateway(id, ip, subnet, gateway) => {
                writer.write_u8(0xA1)?
                    + writer.write_u8(id)?
                    + writer.write_all(&ip)?
                    + writer.write_all(&subnet)?
                    + writer.write_all(&gateway)?
            },

            Request::RetrieveNetworkConfiguration(id) => {
                writer.write_u8(0xFE)?
                    + writer.write_u8(id)?
            },
            Request::RetrieveVersionInformation(id) => {
                writer.write_u8(0xFF)?
                    + writer.write_u8(id)?
            },
        })
    }

    pub fn read(reader: &mut Read) -> Result<Request, Error> {
        Ok(match reader.read_u8()? {
            0x00 => Request::ReadSpecified(reader.read_u8()?, Bus::read(reader)?),
            0x01 => Request::ReadAll(reader.read_u8()?),
            0x02 => Request::ReadAllOnBus(reader.read_u8()?, Bus::read(reader)?),
            0x10 => Request::DiscoverAll(reader.read_u8()?),
            0x11 => Request::DiscoverAllOnBus(reader.read_u8()?, Bus::read(reader)?),

            0xA0 => Request::SetNetworkMac(reader.read_u8()?, [
                reader.read_u8()?, reader.read_u8()?, reader.read_u8()?,
                reader.read_u8()?, reader.read_u8()?, reader.read_u8()?,
            ]),
            0xA1 => Request::SetNetworkIpSubnetGateway(reader.read_u8()?, [
               reader.read_u8()?, reader.read_u8()?, reader.read_u8()?, reader.read_u8()?,
            ], [
                reader.read_u8()?, reader.read_u8()?, reader.read_u8()?, reader.read_u8()?,
            ], [
                reader.read_u8()?, reader.read_u8()?, reader.read_u8()?, reader.read_u8()?,
            ]),

            0xFE => Request::RetrieveNetworkConfiguration(reader.read_u8()?),
            0xFF => Request::RetrieveVersionInformation(reader.read_u8()?),
            _ => return Err(Error::UnknownTypeIdentifier)
        })
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Bus {
    OneWire,
}

impl Bus {
    pub fn write(&self, writer: &mut Write) -> Result<usize, Error> {
        Ok(match self {
            &Bus::OneWire => writer.write_u8(0x00)?,
            _ => return Err(Error::UnknownTypeIdentifier),
        })
    }

    pub fn read(reader: &mut Read) -> Result<Bus, Error> {
        Ok(match reader.read_u8()? {
            0x00 => Bus::OneWire,
            _ => return Err(Error::UnknownTypeIdentifier),
        })
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Response {
    NotImplemented(u8),
    NotAvailable(u8),
    Ok(u8, Format),
}

impl Response {
    pub fn write(&self, writer: &mut Write) -> Result<usize, Error> {
        Ok(match self {
            &Response::NotImplemented(id) => {
                writer.write_u8(0xF0)?
                    + writer.write_u8(id)?
            },
            &Response::NotAvailable(id) => {
                writer.write_u8(0xF1)?
                    + writer.write_u8(id)?
            },
            &Response::Ok(id, format) => {
                writer.write_u8(0x00)?
                    + writer.write_u8(id)?
                    + format.write(writer)?
            },
        })
    }

    pub fn read(reader: &mut Read) -> Result<Response, Error> {
        Ok(match reader.read_u8()? {
            0xF0 => Response::NotImplemented(reader.read_u8()?),
            0xF1 => Response::NotAvailable(reader.read_u8()?),
            0x00 => Response::Ok(reader.read_u8()?, Format::read(reader)?),
            _ => return Err(Error::UnknownTypeIdentifier)
        })
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Format {
    Empty,
    ValueOnly(Type),
    AddressOnly(Type),
    AddressValuePairs(Type, Type)
}

impl Format {
    pub fn write(&self, writer: &mut Write) -> Result<usize, Error> {
        Ok(match self {
            &Format::ValueOnly(t) => {
                writer.write_u8(0x00)?
                    + t.write(writer)?
            },
            &Format::AddressOnly(t) => {
                writer.write_u8(0x01)?
                    + t.write(writer)?
            },
            &Format::AddressValuePairs(t1, t2) => {
                writer.write_u8(0x02)?
                    + t1.write(writer)?
                    + t2.write(writer)?
            },
            &Format::Empty => {
                writer.write_u8(0xFF)?
            },
        })
    }

    pub fn read(reader: &mut Read) -> Result<Format, Error> {
        Ok(match reader.read_u8()? {
            0x00 => Format::ValueOnly(Type::read(reader)?),
            0x01 => Format::AddressOnly(Type::read(reader)?),
            0x02 => Format::AddressValuePairs(Type::read(reader)?, Type::read(reader)?),
            0xFF => Format::Empty,
            _ => return Err(Error::UnknownTypeIdentifier)
        })
    }
}


#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Type {
    F32,
    Bytes(u8),
    String(u8),
}

impl Type {
    pub fn write(&self, writer: &mut Write) -> Result<usize, Error> {
        Ok(match self {
            &Type::F32 => writer.write_u8(0x00)?,
            &Type::Bytes(size) => {
                writer.write_u8(0x01)?
                    + writer.write_u8(size)?
            },
            &Type::String(size) => {
                writer.write_u8(0x02)?
                    + writer.write_u8(size)?
            }
        })
    }

    pub fn read(reader: &mut Read) -> Result<Type, Error> {
        Ok(match reader.read_u8()? {
            0x00 => Type::F32,
            0x01 => Type::Bytes(reader.read_u8()?),
            0x02 => Type::String(reader.read_u8()?),
            _ => return Err(Error::UnknownTypeIdentifier)
        })
    }
}

pub trait Read {
    fn read_u8(&mut self) -> Result<u8, Error>;

    fn available(&self) -> usize;
}

impl<'a> Read for &'a [u8] {
    fn read_u8(&mut self) -> Result<u8, Error> {
        if self.len() < 1 {
            Err(Error::UnexpectedEOF)
        } else {
            let (a, b) = self.split_at(1);
            *self = b;
            Ok(a[0])
        }
    }
    fn available(&self) -> usize {
        self.len()
    }
}

pub trait Write {
    fn write_u8(&mut self, value: u8) -> Result<usize, Error>;

    fn available(&self) -> usize;

    fn write_all(&mut self, bytes: &[u8]) -> Result<usize, Error> {
        if self.available() < bytes.len() {
            Err(Error::BufferToSmall)
        } else {
            for b in bytes {
                self.write_u8(*b)?;
            }
            Ok(bytes.len())
        }
    }
}

impl<'a> Write for &'a mut [u8] {
    fn write_u8(&mut self, value: u8) -> Result<usize, Error> {
        if self.len() < 1 {
            Err(Error::BufferToSmall)
        } else {
            let (a, b) = ::core::mem::replace(self, &mut []).split_at_mut(1);
            a[0] = value;
            *self = b;
            Ok(1)
        }
    }
    fn available(&self) -> usize {
        self.len()
    }
}