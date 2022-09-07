use crate::props::{ComponentRoot, ModuleId, Property, PropertyId, PropertyReportV1};
use crate::{Error, Format, Read, Request, Response, Type, Write};

pub struct ListComponentsResponder {
    pub request_id: u8,
    pub dyn_list_report_v1: bool,
}

impl ListComponentsResponder {
    #[inline]
    pub fn opt_from(request: &Request) -> Option<Self> {
        match request {
            Request::ListComponents(id) | Request::ListComponentsWithReportV1(id) => Some(Self {
                request_id: *id,
                dyn_list_report_v1: matches!(request, Request::ListComponentsWithReportV1(_)),
            }),
            _ => None,
        }
    }

    #[inline]
    pub fn write<P, T, M>(
        &self,
        response_writer: &mut impl Write,
        properties: &[Property<P, T>],
        module_properties: Option<(ModuleId, &[Property<P, M>])>,
    ) -> Result<usize, Error> {
        let available_before = response_writer.available();
        Response::Ok(
            self.request_id,
            if self.dyn_list_report_v1 {
                Format::ValueOnly(Type::DynListPropertyReportV1)
            } else {
                Format::AddressOnly(Type::PropertyId)
            },
        )
        .write(response_writer)?;

        for property in properties {
            if self.dyn_list_report_v1 {
                PropertyReportV1::from(property).write(response_writer)?;
            } else {
                PropertyId::from_slice(property.id).write(response_writer)?;
            }
        }

        if let Some((module_id, module_properties)) = module_properties {
            for property in module_properties {
                let prefix_len = 4;
                let id_len = property.id.len().min((u8::MAX - prefix_len) as usize) as u8;
                let len = prefix_len + id_len;

                response_writer.write_u8(len)?;
                response_writer.write_all(&[
                    ComponentRoot::Module as u8,
                    module_id.group,
                    module_id.id,
                    module_id.ext,
                ])?;
                response_writer.write_all(&property.id[..id_len as usize])?;

                if self.dyn_list_report_v1 {
                    PropertyReportV1::from(property).write_no_id(response_writer)?;
                }
            }
        }

        Ok(available_before - response_writer.available())
    }
}

pub struct RetrievePropertyResponder<'a> {
    pub request_id: u8,
    pub prop_id_len: u8,
    pub payload: &'a mut dyn Read,
}

impl<'a> RetrievePropertyResponder<'a> {
    pub fn opt_from(request: &Request, payload: &'a mut dyn Read) -> Option<Self> {
        if let Request::RetrieveProperty(id, len) = request {
            Some(Self {
                request_id: *id,
                prop_id_len: *len,
                payload,
            })
        } else {
            None
        }
    }

    #[inline]
    pub fn write<P, T, M>(
        self,
        response_writer: &mut impl Write,
        properties: &[Property<P, T>],
        module_properties: Option<(ModuleId, &[Property<P, M>])>,
        p: &mut P,
        t: &mut T,
        m: &mut M,
    ) -> Result<usize, Error> {
        const PID_PATH_MAX_DEPTH: usize = 8_usize;

        let available_before = response_writer.available();
        let len = PID_PATH_MAX_DEPTH.min(usize::from(self.prop_id_len));

        let buffer = {
            let mut buffer = [0u8; PID_PATH_MAX_DEPTH];
            for i in 0..len {
                buffer[i as usize] = self.payload.read_u8()?;
            }
            buffer
        };

        let pid_path = &buffer[..len];
        let module = module_properties.as_ref().map(|(m, _)| m);
        let module_properties = module_properties.as_ref().map(|(_, p)| *p).unwrap_or(&[]);

        match pid_path {
            [component, module_group, module_id, module_ext, prop_id @ ..]
                if *component == ComponentRoot::Module as u8
                    && Some(*module_group) == module.map(|m| m.group)
                    && Some(*module_id) == module.map(|m| m.id)
                    && Some(*module_ext) == module.map(|m| m.ext) =>
            {
                for property in module_properties {
                    if property.id == prop_id {
                        drop(buffer);
                        if let Some(read_fn) = property.read.as_ref() {
                            Response::Ok(
                                self.request_id,
                                Format::ValueOnly(property.type_hint.unwrap_or(Type::DynBytes)),
                            )
                            .write(response_writer)?;
                            read_fn(p, m, response_writer)?;
                        }
                        break;
                    }
                }
            }
            _ => {
                for property in properties {
                    if property.id == pid_path {
                        if let Some(read_fn) = property.read.as_ref() {
                            Response::Ok(
                                self.request_id,
                                Format::ValueOnly(property.type_hint.unwrap_or(Type::DynBytes)),
                            )
                            .write(response_writer)?;
                            read_fn(p, t, response_writer)?;
                        }
                        break;
                    }
                }
            }
        }

        if available_before == response_writer.available() {
            Response::NotAvailable(self.request_id).write(response_writer)?;
        }

        Ok(available_before - response_writer.available())
    }
}
