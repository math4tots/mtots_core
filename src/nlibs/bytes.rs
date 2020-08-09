use crate::ConvertToHandle;
use crate::Globals;
use crate::Handle;
use crate::NativeModule;
use crate::Result;
use crate::Value;
use std::convert::TryFrom;

const NAME: &'static str = "a.bytes";

pub(super) fn new() -> NativeModule {
    NativeModule::new(NAME, |m| {
        m.doc("Utilities for dealing with raw bytes");
        m.class::<Vec<u8>, _>("Bytes", |cls| {
            cls.sfunc("__call", ["data"], "", |globals, args, _| {
                let mut args = args.into_iter();
                let data = args.next().unwrap();
                Ok(data.convert_to_handle::<Vec<u8>>(globals)?.into())
            });
        });
    })
}

impl ConvertToHandle for Vec<u8> {
    fn convert(globals: &mut Globals, value: Value) -> Result<Handle<Vec<u8>>> {
        let mut bytes = Vec::new();
        add_bytes(globals, &mut bytes, &value)?;
        Ok(globals.new_handle(bytes)?)
    }
}

fn add_bytes(globals: &mut Globals, out: &mut Vec<u8>, value: &Value) -> Result<()> {
    match value {
        _ if value.is_handle::<Vec<u8>>() => {
            out.extend(value.clone().into_handle::<Vec<u8>>()?.borrow().iter());
        }
        Value::Number(_) => out.push(u8::try_from(value)?),
        Value::String(string) => out.extend(string.str().as_bytes()),
        _ => {
            value.easy_iter_unpack(|iter| {
                for value in iter {
                    add_bytes(globals, out, &value)?;
                }
                Ok(())
            })?;
        }
    }
    Ok(())
}
