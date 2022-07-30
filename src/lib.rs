#![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
extern crate num_enum;

#[cfg(feature = "std")]
pub mod client;
pub mod props;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Error {
    BufferToSmall,
    UnexpectedEOF,
    UnknownTypeIdentifier,
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

#[cfg(feature = "std")]
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Request {
    ReadSpecified(u8, Bus),
    ReadAll(u8),
    ReadAllOnBus(u8, Bus),
    DiscoverAll(u8),
    DiscoverAllOnBus(u8, Bus),

    SetNetworkMac(u8, [u8; 6]),
    SetNetworkIpSubnetGateway(u8, [u8; 4], [u8; 4], [u8; 4]),

    ListComponents(u8),
    ListComponentsWithReportV1(u8),

    RetrieveProperty(u8, u8),
    RetrieveErrorDump(u8),
    RetrieveDeviceInformation(u8),
    RetrieveNetworkConfiguration(u8),
    RetrieveVersionInformation(u8),
}

impl Request {
    pub fn id(&self) -> u8 {
        match self {
            Request::ReadSpecified(id, _) => *id,
            Request::ReadAll(id) => *id,
            Request::ReadAllOnBus(id, _) => *id,
            Request::DiscoverAll(id) => *id,
            Request::DiscoverAllOnBus(id, _) => *id,
            Request::SetNetworkMac(id, _) => *id,
            Request::SetNetworkIpSubnetGateway(id, _, _, _) => *id,
            Request::ListComponents(id) => *id,
            Request::ListComponentsWithReportV1(id) => *id,
            Request::RetrieveProperty(id, _) => *id,
            Request::RetrieveErrorDump(id) => *id,
            Request::RetrieveDeviceInformation(id) => *id,
            Request::RetrieveNetworkConfiguration(id) => *id,
            Request::RetrieveVersionInformation(id) => *id,
        }
    }

    pub fn write(&self, writer: &mut impl Write) -> Result<usize, Error> {
        Ok(match *self {
            Request::ReadSpecified(id, bus) => {
                writer.write_u8(0x00)? + writer.write_u8(id)? + bus.write(writer)?
            }
            Request::ReadAll(id) => writer.write_u8(0x01)? + writer.write_u8(id)?,
            Request::ReadAllOnBus(id, bus) => {
                writer.write_u8(0x02)? + writer.write_u8(id)? + bus.write(writer)?
            }
            Request::DiscoverAll(id) => writer.write_u8(0x10)? + writer.write_u8(id)?,
            Request::DiscoverAllOnBus(id, bus) => {
                writer.write_u8(0x11)? + writer.write_u8(id)? + bus.write(writer)?
            }

            Request::SetNetworkMac(id, mac) => {
                writer.write_u8(0xA0)? + writer.write_u8(id)? + writer.write_all(&mac)?
            }
            Request::SetNetworkIpSubnetGateway(id, ip, subnet, gateway) => {
                writer.write_u8(0xA1)?
                    + writer.write_u8(id)?
                    + writer.write_all(&ip)?
                    + writer.write_all(&subnet)?
                    + writer.write_all(&gateway)?
            }

            Request::ListComponents(id) => writer.write_u8(0xD0)? + writer.write_u8(id)?,
            Request::ListComponentsWithReportV1(id) => {
                writer.write_u8(0xD1)? + writer.write_u8(id)?
            }

            Request::RetrieveProperty(id, len) => {
                writer.write_u8(0xFB)? + writer.write_u8(id)? + writer.write_u8(len)?
            }

            Request::RetrieveErrorDump(id) => writer.write_u8(0xFC)? + writer.write_u8(id)?,
            Request::RetrieveDeviceInformation(id) => {
                writer.write_u8(0xFD)? + writer.write_u8(id)?
            }
            Request::RetrieveNetworkConfiguration(id) => {
                writer.write_u8(0xFE)? + writer.write_u8(id)?
            }
            Request::RetrieveVersionInformation(id) => {
                writer.write_u8(0xFF)? + writer.write_u8(id)?
            }
        })
    }

    pub fn read(reader: &mut impl Read) -> Result<Request, Error> {
        Ok(match reader.read_u8()? {
            0x00 => Request::ReadSpecified(reader.read_u8()?, Bus::read(reader)?),
            0x01 => Request::ReadAll(reader.read_u8()?),
            0x02 => Request::ReadAllOnBus(reader.read_u8()?, Bus::read(reader)?),
            0x10 => Request::DiscoverAll(reader.read_u8()?),
            0x11 => Request::DiscoverAllOnBus(reader.read_u8()?, Bus::read(reader)?),

            0xA0 => Request::SetNetworkMac(
                reader.read_u8()?,
                [
                    reader.read_u8()?,
                    reader.read_u8()?,
                    reader.read_u8()?,
                    reader.read_u8()?,
                    reader.read_u8()?,
                    reader.read_u8()?,
                ],
            ),
            0xA1 => Request::SetNetworkIpSubnetGateway(
                reader.read_u8()?,
                [
                    reader.read_u8()?,
                    reader.read_u8()?,
                    reader.read_u8()?,
                    reader.read_u8()?,
                ],
                [
                    reader.read_u8()?,
                    reader.read_u8()?,
                    reader.read_u8()?,
                    reader.read_u8()?,
                ],
                [
                    reader.read_u8()?,
                    reader.read_u8()?,
                    reader.read_u8()?,
                    reader.read_u8()?,
                ],
            ),

            0xD0 => Request::ListComponents(reader.read_u8()?),
            0xD1 => Request::ListComponentsWithReportV1(reader.read_u8()?),

            0xFB => Request::RetrieveProperty(reader.read_u8()?, reader.read_u8()?),
            0xFC => Request::RetrieveErrorDump(reader.read_u8()?),
            0xFD => Request::RetrieveDeviceInformation(reader.read_u8()?),
            0xFE => Request::RetrieveNetworkConfiguration(reader.read_u8()?),
            0xFF => Request::RetrieveVersionInformation(reader.read_u8()?),
            _ => return Err(Error::UnknownTypeIdentifier),
        })
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Bus {
    OneWire,
    I2C,
    Custom(u8),
}

impl Bus {
    pub fn write(&self, writer: &mut impl Write) -> Result<usize, Error> {
        Ok(match self {
            Bus::OneWire => writer.write_u8(0x00)?,
            Bus::I2C => writer.write_u8(0x01)?,
            Bus::Custom(id) => writer.write_u8(0xFF)? + writer.write_u8(*id)?,
        })
    }

    pub fn read(reader: &mut impl Read) -> Result<Bus, Error> {
        Ok(match reader.read_u8()? {
            0x00 => Bus::OneWire,
            0x01 => Bus::I2C,
            0xFF => Bus::Custom(reader.read_u8()?),
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
    pub fn id(&self) -> u8 {
        match self {
            Response::NotImplemented(id) => *id,
            Response::NotAvailable(id) => *id,
            Response::Ok(id, _) => *id,
        }
    }

    pub fn write(&self, writer: &mut impl Write) -> Result<usize, Error> {
        Ok(match self {
            Response::NotImplemented(id) => writer.write_u8(0xF0)? + writer.write_u8(*id)?,
            Response::NotAvailable(id) => writer.write_u8(0xF1)? + writer.write_u8(*id)?,
            Response::Ok(id, format) => {
                writer.write_u8(0x00)? + writer.write_u8(*id)? + format.write(writer)?
            }
        })
    }

    pub fn read(reader: &mut impl Read) -> Result<Response, Error> {
        Ok(match reader.read_u8()? {
            0xF0 => Response::NotImplemented(reader.read_u8()?),
            0xF1 => Response::NotAvailable(reader.read_u8()?),
            0x00 => Response::Ok(reader.read_u8()?, Format::read(reader)?),
            _ => return Err(Error::UnknownTypeIdentifier),
        })
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Format {
    Empty,
    ValueOnly(Type),
    AddressOnly(Type),
    AddressValuePairs(Type, Type),
}

impl Format {
    pub fn write(&self, writer: &mut impl Write) -> Result<usize, Error> {
        Ok(match self {
            Format::ValueOnly(t) => writer.write_u8(0x00)? + t.write(writer)?,
            Format::AddressOnly(t) => writer.write_u8(0x01)? + t.write(writer)?,
            Format::AddressValuePairs(t1, t2) => {
                writer.write_u8(0x02)? + t1.write(writer)? + t2.write(writer)?
            }
            Format::Empty => writer.write_u8(0xFF)?,
        })
    }

    pub fn read(reader: &mut impl Read) -> Result<Format, Error> {
        Ok(match reader.read_u8()? {
            0x00 => Format::ValueOnly(Type::read(reader)?),
            0x01 => Format::AddressOnly(Type::read(reader)?),
            0x02 => Format::AddressValuePairs(Type::read(reader)?, Type::read(reader)?),
            0xFF => Format::Empty,
            _ => return Err(Error::UnknownTypeIdentifier),
        })
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Type {
    F32,
    Bytes(u8),
    String(u8),
    PropertyId,
    DynString,
    DynBytes,

    DynListPropertyReportV1,

    U128,
    I128,
    U64,
    I64,
    U32,
    I32,
    U16,
    I16,
    U8,
    I8,
}

impl Type {
    pub fn write(&self, writer: &mut dyn Write) -> Result<usize, Error> {
        Ok(match self {
            Type::F32 => writer.write_u8(0x00)?,
            Type::Bytes(size) => writer.write_u8(0x01)? + writer.write_u8(*size)?,
            Type::String(size) => writer.write_u8(0x02)? + writer.write_u8(*size)?,
            Type::PropertyId => writer.write_u8(0x03)?,
            Type::DynString => writer.write_u8(0x04)?,
            Type::DynBytes => writer.write_u8(0x05)?,

            Type::DynListPropertyReportV1 => writer.write_u8(0xC0)?,

            Type::U128 => writer.write_u8(0xF6)?,
            Type::I128 => writer.write_u8(0xF7)?,
            Type::U64 => writer.write_u8(0xF8)?,
            Type::I64 => writer.write_u8(0xF9)?,
            Type::U32 => writer.write_u8(0xFA)?,
            Type::I32 => writer.write_u8(0xFB)?,
            Type::U16 => writer.write_u8(0xFC)?,
            Type::I16 => writer.write_u8(0xFD)?,
            Type::U8 => writer.write_u8(0xFE)?,
            Type::I8 => writer.write_u8(0xFF)?,
        })
    }

    pub fn read(reader: &mut dyn Read) -> Result<Type, Error> {
        Ok(match reader.read_u8()? {
            0x00 => Type::F32,
            0x01 => Type::Bytes(reader.read_u8()?),
            0x02 => Type::String(reader.read_u8()?),
            0x03 => Type::PropertyId,
            0x04 => Type::DynString,
            0x05 => Type::DynBytes,

            0xC0 => Type::DynListPropertyReportV1,

            0xF6 => Type::U128,
            0xF7 => Type::I128,
            0xF8 => Type::U64,
            0xF9 => Type::I64,
            0xFA => Type::U32,
            0xFB => Type::I32,
            0xFC => Type::U16,
            0xFD => Type::I16,
            0xFE => Type::U8,
            0xFF => Type::I8,

            _ => return Err(Error::UnknownTypeIdentifier),
        })
    }
}

pub trait Read {
    fn read_u8(&mut self) -> Result<u8, Error>;

    fn read_all(&mut self, destination: &mut [u8]) -> Result<u8, Error> {
        let len = destination.len().min(u8::MAX as usize) as u8;
        if self.available() < usize::from(len) {
            Err(Error::UnexpectedEOF)
        } else {
            for destination in destination.iter_mut().take(usize::from(len)) {
                *destination = self.read_u8()?;
            }
            Ok(len)
        }
    }

    fn available(&self) -> usize;
}

impl<'a> Read for &'a [u8] {
    fn read_u8(&mut self) -> Result<u8, Error> {
        if self.is_empty() {
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
        if self.is_empty() {
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

#[cfg(feature = "std")]
impl Write for Vec<u8> {
    fn write_u8(&mut self, value: u8) -> Result<usize, Error> {
        self.push(value);
        Ok(1)
    }

    fn available(&self) -> usize {
        usize::MAX - self.len()
    }

    fn write_all(&mut self, bytes: &[u8]) -> Result<usize, Error> {
        self.extend_from_slice(bytes);
        Ok(bytes.len())
    }
}
