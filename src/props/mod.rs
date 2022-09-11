use crate::{Error, Read, Type, Write};
use core::num::NonZeroU16;

pub mod handling;

#[macro_export]
macro_rules! property_read_fn {
    (|$platform:ident: &mut $platformTy:ty, $module:ident: &mut $moduleTy:ty, $write:ident| $body:expr) => {{
        Some(
            |$platform: &mut $platformTy,
             $module: &mut $moduleTy,
             $write: &mut dyn $crate::Write|
             -> Result<usize, $crate::Error> {
                {
                    let _ = &($platform);
                    let _ = &($module);
                    let _ = &($write);
                };
                $body
            },
        )
    }};
    (|$platform:ident, $module:ident: &mut $moduleTy:ty, $write:ident| $body:expr) => {
        property_read_fn! { |$platform: &mut _, $module: &mut $moduleTy, $write| $body }
    };
    (|$platform:ident, $write:ident| $body:expr) => {
        property_read_fn! { |$platform: &mut _, _t: &mut (), $write| $body }
    };
}

#[macro_export]
macro_rules! property_write_fn {
    (|$platform:ident: &mut $platformTy:ty, $module:ident: &mut $moduleTy:ty, $read:ident| $body:expr) => {{
        Some(
            |$platform: &mut $platformTy,
             $module: &mut $moduleTy,
             $read: &mut dyn $crate::Read|
             -> Result<usize, $crate::Error> {
                {
                    let _ = &($platform);
                    let _ = &($module);
                    let _ = &($read);
                };
                $body
            },
        )
    }};
    (|$platform:ident, $module:ident: &mut $moduleTy:ty, $read:ident| $body:expr) => {
        property_write_fn! { |$platform: &mut _, $module: &mut $moduleTy, $read| $body }
    };
    (|$platform:ident, $read:ident| $body:expr) => {
        property_write_fn! { |$platform, _t: &mut (), $read| $body }
    };
}

#[repr(u8)]
#[derive(Copy, Clone, TryFromPrimitive)]
pub enum ComponentRoot {
    Device = 0x10,
    System = 0x20,
    Platform = 0x30,
    Module = 0x40,
}

pub enum SystemComponent {
    Whatever,
}

#[repr(u8)]
#[derive(Copy, Clone, TryFromPrimitive)]
pub enum DeviceComponent {
    Cpu = 0x00,
    Frequency = 0x01,
    Uptime = 0x02,
}

#[repr(u8)]
#[derive(Copy, Clone, TryFromPrimitive)]
pub enum CpuComponent {
    Id = 0x00,
    Implementer = 0x01,
    Variant = 0x02,
    PartNumber = 0x03,
    Revision = 0x04,
}

impl CpuComponent {
    pub const fn to_cid_path(self) -> [u8; 3] {
        [
            ComponentRoot::Device as u8,
            DeviceComponent::Cpu as u8,
            self as u8,
        ]
    }
}

pub enum PlatformComponent {
    Meta = 0x00,
    EeeProm = 0x10,
    Network = 0x11,
    Temperature = 0x12,
    Sntp = 0x13,
}

pub enum MetaInformation {
    Version = 0x00,
    // Module = 0x10,
}

pub enum EeePromComponent {
    MagicCrcStart = 0x10,
}

pub enum NetworkComponent {
    Mac = 0x10,
    Ip = 0x11,
    Subnet = 0x12,
    Gateway = 0x13,
}

pub enum TemperatureComponent {
    Value = 0x00,
}

pub enum SntpComponent {
    CurrentTimeMillis = 0x00,
    LastOffsetMillis = 0x01,
    LastUpdateMillis = 0x02,
}

pub struct ModuleId {
    pub group: u8,
    pub id: u8,
    pub ext: u8,
}

pub enum ModuleComponent<'a> {
    Other(&'a [u8]),
}

pub struct PropertyId<'a>(&'a [u8]);

impl PropertyId<'_> {
    pub fn write(&self, writer: &mut impl crate::Write) -> Result<usize, crate::Error> {
        let data = self.0;
        let len = data.len().min(u8::MAX as usize) as u8;
        Ok(writer.write_u8(len)? + writer.write_all(&data[..usize::from(len)])?)
    }
}

impl<'a> PropertyId<'a> {
    pub const fn from_slice(slice: &'a [u8]) -> Self {
        Self(slice)
    }
}

impl<'a> From<&'a [u8]> for PropertyId<'a> {
    fn from(slice: &'a [u8]) -> Self {
        Self::from_slice(slice)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum QueryComplexity {
    Unknown,
    Low {
        estimated_millis: Option<NonZeroU16>,
    },
    High {
        estimated_millis: Option<NonZeroU16>,
    },
}

impl QueryComplexity {
    pub const fn high() -> Self {
        QueryComplexity::High {
            estimated_millis: None,
        }
    }

    pub const fn low() -> Self {
        QueryComplexity::Low {
            estimated_millis: None,
        }
    }

    pub fn read(reader: &mut impl crate::Read) -> Result<Self, crate::Error> {
        Ok(match reader.read_u8()? {
            0x10 => {
                let mut millis = 0u16.to_be_bytes();
                reader.read_all(millis.as_mut())?;
                Self::Low {
                    estimated_millis: NonZeroU16::new(u16::from_be_bytes(millis)),
                }
            }
            0x20 => {
                let mut millis = 0u16.to_be_bytes();
                reader.read_all(millis.as_mut())?;
                Self::High {
                    estimated_millis: NonZeroU16::new(u16::from_be_bytes(millis)),
                }
            }
            _id => return Err(crate::Error::UnknownTypeIdentifier),
        })
    }

    pub fn write(&self, writer: &mut dyn crate::Write) -> Result<usize, crate::Error> {
        match self {
            QueryComplexity::Unknown => writer.write_u8(0x00),
            QueryComplexity::Low { estimated_millis } => {
                writer.write_u8(0x10)?;
                writer.write_all(
                    &estimated_millis
                        .map(|n| n.get().to_be_bytes())
                        .unwrap_or_default(),
                )
            }
            QueryComplexity::High { estimated_millis } => {
                writer.write_u8(0x20)?;
                writer.write_all(
                    &estimated_millis
                        .map(|n| n.get().to_be_bytes())
                        .unwrap_or_default(),
                )
            }
        }
    }
}

pub type ReadFn<P, T> = fn(&mut P, &mut T, &mut dyn Write) -> Result<usize, Error>;
pub type WriteFn<P, T> = fn(&mut P, &mut T, &mut dyn Read) -> Result<usize, Error>;

pub struct Property<P, T> {
    pub id: &'static [u8],
    pub type_hint: Option<Type>,
    pub description: Option<&'static str>,
    pub complexity: QueryComplexity,
    pub read: Option<ReadFn<P, T>>,
    pub write: Option<WriteFn<P, T>>,
}

#[derive(Debug)]
pub struct PropertyReportV1 {
    #[cfg(feature = "std")]
    pub id: Vec<u8>,
    #[cfg(not(feature = "std"))]
    pub id: &'static [u8],
    pub type_hint: Option<Type>,
    #[cfg(feature = "std")]
    pub description: Option<String>,
    #[cfg(not(feature = "std"))]
    pub description: Option<&'static str>,
    pub complexity: QueryComplexity,
    pub read: bool,
    pub write: bool,
}

impl PropertyReportV1 {
    pub fn write(&self, writer: &mut dyn Write) -> Result<usize, Error> {
        let id_len = self.id.len().min(u8::MAX as usize);
        Ok(writer.write_u8(id_len as u8)?
            + writer.write_all(&self.id[..id_len])?
            + self.write_no_id(writer)?)
    }

    pub fn write_no_id(&self, writer: &mut dyn Write) -> Result<usize, Error> {
        let header = 0x00u8
            | self.type_hint.map(|_| 1u8 << 7).unwrap_or_default()
            | self
                .description
                .as_ref()
                .map(|_| 1u8 << 6)
                .unwrap_or_default()
            | if self.read { 1u8 << 5 } else { 0u8 }
            | if self.write { 1u8 << 4 } else { 0u8 };

        Ok(writer.write_u8(header)?
            + if let Some(ty) = self.type_hint {
                ty.write(writer)?
            } else {
                0
            }
            + if let Some(desc) = self.description.as_deref() {
                let len = desc.len().min(u8::MAX as usize);
                writer.write_u8(len as u8)? + writer.write_all(&desc.as_bytes()[..len])?
            } else {
                0
            }
            + self.complexity.write(writer)?)
    }

    #[cfg(feature = "std")]
    pub fn read(reader: &mut impl Read) -> Result<Self, Error> {
        let id = {
            let id_len = usize::from(reader.read_u8()?);
            let mut vec = core::iter::repeat(0u8).take(id_len).collect::<Vec<u8>>();
            reader.read_all(&mut vec[..])?;
            vec
        };

        let header = reader.read_u8()?;
        let ty = if header & (1u8 << 7) != 0 {
            Some(Type::read(reader)?)
        } else {
            None
        };

        let desc = if header & (1u8 << 6) != 0 {
            let desc_len = usize::from(reader.read_u8()?);
            let mut vec = core::iter::repeat(0u8).take(desc_len).collect::<Vec<u8>>();
            reader.read_all(&mut vec[..])?;
            Some(String::from_utf8_lossy(&vec).to_string())
        } else {
            None
        };

        let complexity = QueryComplexity::read(reader)?;
        Ok(PropertyReportV1 {
            id,
            type_hint: ty,
            description: desc,
            complexity,
            read: header & (1u8 << 5) != 0,
            write: header & (1u8 << 4) != 0,
        })
    }

    #[cfg(feature = "std")]
    pub fn id_formatted(&self) -> String {
        let mut string = String::with_capacity(self.id.len() * 3 - 1);
        for (i, id) in self.id.iter().enumerate() {
            if i > 0 {
                string.push(':');
            }

            use std::fmt::Write;
            write!(&mut string, "{:02x}", *id).unwrap();
        }
        string
    }
}

impl<P, T> From<&Property<P, T>> for PropertyReportV1 {
    fn from(property: &Property<P, T>) -> Self {
        PropertyReportV1 {
            id: property.id.into(),
            type_hint: property.type_hint,
            description: property.description.map(Into::into),
            complexity: property.complexity,
            read: property.read.is_some(),
            write: property.write.is_some(),
        }
    }
}
