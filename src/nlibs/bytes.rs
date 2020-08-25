use crate::ArgSpec;
use crate::ConvertValue;
use crate::Globals;
use crate::IntType;
use crate::NativeModule;
use crate::Result;
use crate::Value;
use std::convert::TryFrom;

const NAME: &'static str = "a.bytes";

pub(super) fn new() -> NativeModule {
    NativeModule::new(NAME, |m| {
        m.doc("Utilities for dealing with raw bytes");
        m.dep("a.int", None, &[]);
        m.class::<Vec<u8>, _>("Bytes", |cls| {
            cls.eq(|a, b| a == b);
            cls.repr(|owner| {
                let mut s = String::new();
                s.push_str("Bytes([");
                let mut first = true;
                for b in owner {
                    if !first {
                        s.push_str(", ");
                    }
                    s.push_str(&format!("{}", b));
                    first = false;
                }
                s.push_str("])");
                s.into()
            });
            cls.sfunc("__call", ["data"], "", |globals, args, _| {
                let mut args = args.into_iter();
                let data = args.next().unwrap();
                let data = data.convert::<Vec<u8>>(globals)?;
                globals.new_handle(data).map(Value::from)
            });
            cls.sfunc(
                "le",
                ["type", "int"],
                "Given an int type, converts an int value into little endian bytes",
                |globals, args, _| {
                    let mut args = args.into_iter();
                    let type_ = args.next().unwrap().convert::<IntType>(globals)?;
                    let int = args.next().unwrap().f64()?;
                    let bytes = le_bytes_from_int(globals, type_, int)?;
                    globals.new_handle(bytes).map(Value::from)
                },
            );
            cls.sfunc(
                "be",
                ["type", "int"],
                "Given an int type, converts an int value into big endian bytes",
                |globals, args, _| {
                    let mut args = args.into_iter();
                    let type_ = args.next().unwrap().convert::<IntType>(globals)?;
                    let int = args.next().unwrap().f64()?;
                    let bytes = be_bytes_from_int(globals, type_, int)?;
                    globals.new_handle(bytes).map(Value::from)
                },
            );
            cls.ifunc(
                "__getitem",
                ["i"],
                "Gets the byte at the given index",
                |owner, _globals, args, _| {
                    let owner = owner.borrow();
                    let mut args = args.into_iter();
                    let i = args.next().unwrap().to_index(owner.len())?;
                    Ok(Value::from(owner[i]))
                },
            );
            cls.ifunc(
                "__slice",
                ["start", "end"],
                "Returns a slice of bytes from the given range",
                |owner, globals, args, _| {
                    let owner = owner.borrow();
                    let mut args = args.into_iter();
                    let len = owner.len();
                    let start = args.next().unwrap().to_start_index(len)?;
                    let end = args.next().unwrap().to_end_index(len)?;
                    Ok(globals
                        .new_handle::<Vec<u8>>(owner[start..end].to_vec())?
                        .into())
                },
            );
            cls.ifunc(
                "le",
                ArgSpec::builder().req("type").def("i", ()),
                concat!(
                    "Converts the given bytes (interpretered in little endian) to an int.\n",
                    "If 'i' is specified, it will start at the given offset ",
                    "and ignore any extra bytes.\n",
                    "Otherwise, an exact number of bytes is expected for the ",
                    "given int type\n",
                ),
                |owner, globals, args, _| {
                    let owner = owner.borrow();
                    let mut args = args.into_iter();
                    let type_ = args.next().unwrap().convert::<IntType>(globals)?;
                    let i = args.next().unwrap();
                    let slice: &[u8] = if i.is_nil() {
                        &owner
                    } else {
                        let i = i.to_index(owner.len())?;
                        &owner[i..i + type_.nbytes()]
                    };
                    Ok(Value::from(type_.from_le_slice(slice)?))
                },
            );
            cls.ifunc(
                "be",
                ArgSpec::builder().req("type").def("i", ()),
                concat!(
                    "Converts the given bytes (interpretered in big endian) to an int.\n",
                    "If 'i' is specified, it will start at the given offset ",
                    "and ignore any extra bytes.\n",
                    "Otherwise, an exact number of bytes is expected for the ",
                    "given int type\n",
                ),
                |owner, globals, args, _| {
                    let owner = owner.borrow();
                    let mut args = args.into_iter();
                    let type_ = args.next().unwrap().convert::<IntType>(globals)?;
                    let i = args.next().unwrap();
                    let slice: &[u8] = if i.is_nil() {
                        &owner
                    } else {
                        let i = i.to_index(owner.len())?;
                        &owner[i..i + type_.nbytes()]
                    };
                    Ok(Value::from(type_.from_be_slice(slice)?))
                },
            );
        });
    })
}

impl ConvertValue for Vec<u8> {
    fn convert(globals: &mut Globals, value: &Value) -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        add_bytes(globals, &mut bytes, value)?;
        Ok(bytes)
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

fn le_bytes_from_int(_globals: &mut Globals, type_: IntType, int: f64) -> Result<Vec<u8>> {
    match type_ {
        IntType::u8 => Ok(vec![int as u8]),
        IntType::i8 => Ok(vec![int as i8 as u8]),
        IntType::u16 => Ok((int as u16).to_le_bytes().to_vec()),
        IntType::i16 => Ok((int as i16).to_le_bytes().to_vec()),
        IntType::u32 => Ok((int as u32).to_le_bytes().to_vec()),
        IntType::i32 => Ok((int as i32).to_le_bytes().to_vec()),
    }
}

fn be_bytes_from_int(_globals: &mut Globals, type_: IntType, int: f64) -> Result<Vec<u8>> {
    match type_ {
        IntType::u8 => Ok(vec![int as u8]),
        IntType::i8 => Ok(vec![int as i8 as u8]),
        IntType::u16 => Ok((int as u16).to_be_bytes().to_vec()),
        IntType::i16 => Ok((int as i16).to_be_bytes().to_vec()),
        IntType::u32 => Ok((int as u32).to_be_bytes().to_vec()),
        IntType::i32 => Ok((int as i32).to_be_bytes().to_vec()),
    }
}
