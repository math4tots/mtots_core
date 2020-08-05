use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::NativeFunction;
use crate::ParameterInfo;
use crate::Symbol;
use crate::Value;
use std::rc::Rc;

pub(super) fn mkcls(base: Rc<Class>) -> Rc<Class> {
    let methods = vec![
        NativeFunction::new(
            "decode",
            &["self", "encoding"],
            None,
            |globals, args, _kwargs| {
                let bytes = Eval::expect_bytes(globals, &args[0])?;
                let encoding = Eval::expect_symbol(globals, &args[1])?;
                if encoding == Symbol::UTF8 {
                    match std::str::from_utf8(bytes) {
                        Ok(s) => Ok(s.into()),
                        Err(error) => globals.set_utf8_error(error),
                    }
                } else {
                    globals.set_exc_str(&format!("Unrecognized encoding {:?}", encoding,))
                }
            },
        ),
        NativeFunction::new("len", &["self"], None, |globals, args, _kwargs| {
            let bytes = Eval::expect_bytes(globals, &args[0])?;
            Ok((bytes.len() as i64).into())
        }),
        NativeFunction::new(
            "__getitem",
            &["self", "i"],
            None,
            |globals, args, _kwargs| {
                let bytes = Eval::expect_bytes(globals, &args[0])?;
                let i = Eval::expect_index(globals, &args[1], bytes.len())?;
                Ok((bytes[i] as i64).into())
            },
        ),
        NativeFunction::new(
            "int",
            ParameterInfo::builder()
                .required("self")
                .required("nbytes")
                .required("i")
                .optional("endian", ()),
            None,
            |globals, args, _kwargs| {
                use std::convert::TryInto;
                let bytes = Eval::expect_bytes(globals, &args[0])?;
                let nbytes = Eval::expect_usize(globals, &args[1])?;
                let i = Eval::expect_usize(globals, &args[2])?;
                if bytes.len() < i + nbytes {
                    return globals.set_exc_str(&format!(
                        "Tried to read {} bytes from {}, but entire buffer is only {} bytes",
                        nbytes,
                        i,
                        bytes.len(),
                    ));
                }
                let endian = if let Value::Nil = &args[3] {
                    Endian::Little
                } else {
                    let symbol = Eval::expect_symbol(globals, &args[3])?;
                    if symbol == globals.symbol_little() {
                        Endian::Little
                    } else if symbol == globals.symbol_big() {
                        Endian::Big
                    } else {
                        return globals.set_exc_str("endianness must be :big or :little");
                    }
                };
                match (nbytes, endian) {
                    (1, _) => Ok((bytes[i] as i8 as i64).into()),
                    (2, Endian::Little) => {
                        Ok((i16::from_le_bytes(bytes[i..i + 2].try_into().unwrap()) as i64).into())
                    }
                    (2, Endian::Big) => {
                        Ok((i16::from_be_bytes(bytes[i..i + 2].try_into().unwrap()) as i64).into())
                    }
                    (4, Endian::Little) => {
                        Ok((i32::from_le_bytes(bytes[i..i + 4].try_into().unwrap()) as i64).into())
                    }
                    (4, Endian::Big) => {
                        Ok((i32::from_be_bytes(bytes[i..i + 4].try_into().unwrap()) as i64).into())
                    }
                    (8, Endian::Little) => {
                        Ok((i64::from_le_bytes(bytes[i..i + 8].try_into().unwrap()) as i64).into())
                    }
                    (8, Endian::Big) => {
                        Ok((i64::from_be_bytes(bytes[i..i + 8].try_into().unwrap()) as i64).into())
                    }
                    _ => globals.set_exc_str(&format!(
                        "nbytes for int must be 1, 2, 4 or 8 but got {}",
                        nbytes
                    )),
                }
            },
        ),
        NativeFunction::new(
            "uint",
            ParameterInfo::builder()
                .required("self")
                .required("nbytes")
                .required("i")
                .optional("endian", ()),
            None,
            |globals, args, _kwargs| {
                use std::convert::TryInto;
                let bytes = Eval::expect_bytes(globals, &args[0])?;
                let nbytes = Eval::expect_usize(globals, &args[1])?;
                let i = Eval::expect_usize(globals, &args[2])?;
                if bytes.len() < i + nbytes {
                    return globals.set_exc_str(&format!(
                        "Tried to read {} bytes from {}, but entire buffer is only {} bytes",
                        nbytes,
                        i,
                        bytes.len(),
                    ));
                }
                let endian = if let Value::Nil = &args[3] {
                    Endian::Little
                } else {
                    let symbol = Eval::expect_symbol(globals, &args[3])?;
                    if symbol == globals.symbol_little() {
                        Endian::Little
                    } else if symbol == globals.symbol_big() {
                        Endian::Big
                    } else {
                        return globals.set_exc_str("endianness must be :big or :little");
                    }
                };
                // nbytes = 8 for uint may not fit in i64
                match (nbytes, endian) {
                    (1, _) => Ok((bytes[i] as i64).into()),
                    (2, Endian::Little) => {
                        Ok((u16::from_le_bytes(bytes[i..i + 2].try_into().unwrap()) as i64).into())
                    }
                    (2, Endian::Big) => {
                        Ok((u16::from_be_bytes(bytes[i..i + 2].try_into().unwrap()) as i64).into())
                    }
                    (4, Endian::Little) => {
                        Ok((u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap()) as i64).into())
                    }
                    (4, Endian::Big) => {
                        Ok((u32::from_be_bytes(bytes[i..i + 4].try_into().unwrap()) as i64).into())
                    }
                    _ => globals.set_exc_str(&format!(
                        "nbytes for uint must be 1, 2 or 4 but got {}",
                        nbytes
                    )),
                }
            },
        ),
        NativeFunction::new(
            "__slice",
            &["self", "start", "end"],
            "Creates a new bytes object consisting of a subrange of this object",
            |globals, args, _kwargs| {
                let bytes = Eval::expect_bytes(globals, &args[0])?;
                let (start, end) =
                    Eval::expect_range_indices(globals, &args[1], &args[2], bytes.len())?;
                Ok((*bytes)[start..end].to_vec().into())
            },
        ),
    ]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
    .collect();

    let static_methods = vec![
        NativeFunction::new("__call", &["pattern"], None, |globals, args, _kwargs| {
            let bytes = Eval::expect_bytes_from_pattern(globals, &args[0])?;
            Ok(bytes.into())
        }),
        NativeFunction::new(
            "le",
            &["n", "val"],
            concat!(
                "Create little endian bytes from an integer or float\n",
                "The first parameter n specifies the number of bytes to use\n",
                "The second parameter specifies the actual value to encode\n",
                "n must be one of 1, 2, 4, 8\n",
            ),
            |globals, args, _kwargs| {
                let n = Eval::expect_uint(globals, &args[0])?;
                let i = Eval::expect_int(globals, &args[1])?;
                match n {
                    1 => {
                        let bytes: &[u8] = &if i < 0 {
                            Eval::check_i8(globals, i)?.to_le_bytes()
                        } else {
                            Eval::check_u8(globals, i)?.to_le_bytes()
                        };
                        Ok(bytes.to_vec().into())
                    }
                    2 => {
                        let bytes: &[u8] = &if i < 0 {
                            Eval::check_i16(globals, i)?.to_le_bytes()
                        } else {
                            Eval::check_u16(globals, i)?.to_le_bytes()
                        };
                        Ok(bytes.to_vec().into())
                    }
                    4 => {
                        let bytes: &[u8] = &if i < 0 {
                            Eval::check_i32(globals, i)?.to_le_bytes()
                        } else {
                            Eval::check_u32(globals, i)?.to_le_bytes()
                        };
                        Ok(bytes.to_vec().into())
                    }
                    8 => {
                        let bytes: &[u8] = &i.to_le_bytes();
                        Ok(bytes.to_vec().into())
                    }
                    _ => globals.set_exc_str(&format!("n must be 1, 2, 4 or 8, but got {}", n,)),
                }
            },
        ),
        NativeFunction::new(
            "be",
            &["n", "val"],
            concat!(
                "Create big endian bytes from an integer or float\n",
                "The first parameter n specifies the number of bytes to use\n",
                "The second parameter specifies the actual value to encode\n",
                "For integers, n must be one of 1, 2, 4, 8\n",
                "For floats, n must be one of 4 or 8\n",
            ),
            |globals, args, _kwargs| {
                let n = Eval::expect_uint(globals, &args[0])?;
                let i = Eval::expect_int(globals, &args[1])?;
                match n {
                    1 => {
                        let bytes: &[u8] = &if i < 0 {
                            Eval::check_i8(globals, i)?.to_be_bytes()
                        } else {
                            Eval::check_u8(globals, i)?.to_be_bytes()
                        };
                        Ok(bytes.to_vec().into())
                    }
                    2 => {
                        let bytes: &[u8] = &if i < 0 {
                            Eval::check_i16(globals, i)?.to_be_bytes()
                        } else {
                            Eval::check_u16(globals, i)?.to_be_bytes()
                        };
                        Ok(bytes.to_vec().into())
                    }
                    4 => {
                        let bytes: &[u8] = &if i < 0 {
                            Eval::check_i32(globals, i)?.to_be_bytes()
                        } else {
                            Eval::check_u32(globals, i)?.to_be_bytes()
                        };
                        Ok(bytes.to_vec().into())
                    }
                    8 => {
                        let bytes: &[u8] = &i.to_be_bytes();
                        Ok(bytes.to_vec().into())
                    }
                    _ => globals.set_exc_str(&format!("n must be 1, 2, 4 or 8, but got {}", n,)),
                }
            },
        ),
        NativeFunction::new(
            "from_iterable",
            &["iterable"],
            None,
            |globals, args, _kwargs| Eval::bytes_from_iterable(globals, &args[0]),
        ),
    ]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
    .collect();

    Class::new0(
        ClassKind::NativeClass,
        "Bytes".into(),
        vec![base],
        None,
        methods,
        static_methods,
    )
    .into()
}

enum Endian {
    Little,
    Big,
}
