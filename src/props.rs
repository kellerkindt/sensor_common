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
    pub const fn to_cid_path(&self) -> [u8; 3] {
        [
            ComponentRoot::Device as u8,
            DeviceComponent::Cpu as u8,
            *self as u8,
        ]
    }
}

pub enum PlatformComponent {}

pub struct ModuleId {
    group: u8,
    id: u8,
    ext: u8,
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
