use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::NativeFunction;
use crate::Symbol;
use crate::SymbolRegistryHandle;
use crate::Value;
use std::rc::Rc;

pub(super) fn mkcls(sr: &SymbolRegistryHandle, base: Rc<Class>) -> Rc<Class> {
    let methods = vec![
        NativeFunction::simple0(
            sr,
            "decode",
            &["self", "encoding"],
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
        NativeFunction::sdnew(
            sr,
            "len",
            (&["self"], &[], None, None),
            None,
            |globals, args, _kwargs| {
                let bytes = Eval::expect_bytes(globals, &args[0])?;
                Ok((bytes.len() as i64).into())
            },
        ),
    ]
    .into_iter()
    .map(|f| (sr.intern_rcstr(f.name()), Value::from(f)))
    .collect();

    let static_methods = vec![
        NativeFunction::simple0(sr, "__call", &["pattern"], |globals, args, _kwargs| {
            let bytes = Eval::expect_bytes_from_pattern(globals, &args[0])?;
            Ok(bytes.into())
        }),
        NativeFunction::sdnew(
            sr,
            "le",
            (&["n", "val"], &[], None, None),
            Some(concat!(
                "Create little endian bytes from an integer or float\n",
                "The first parameter n specifies the number of bytes to use\n",
                "The second parameter specifies the actual value to encode\n",
                "For integers, n must be one of 1, 2, 4, 8\n",
                "For floats, n must be ine of 4 or 8\n",
            )),
            |globals, args, _kwargs| {
                let n = Eval::expect_uint(globals, &args[0])?;
                match (n, &args[1]) {
                    (1, Value::Int(i)) => {
                        let bytes: &[u8] = &if *i < 0 {
                            Eval::check_i8(globals, *i)?.to_le_bytes()
                        } else {
                            Eval::check_u8(globals, *i)?.to_le_bytes()
                        };
                        Ok(bytes.to_vec().into())
                    }
                    (2, Value::Int(i)) => {
                        let bytes: &[u8] = &if *i < 0 {
                            Eval::check_i16(globals, *i)?.to_le_bytes()
                        } else {
                            Eval::check_u16(globals, *i)?.to_le_bytes()
                        };
                        Ok(bytes.to_vec().into())
                    }
                    (4, Value::Int(i)) => {
                        let bytes: &[u8] = &if *i < 0 {
                            Eval::check_i32(globals, *i)?.to_le_bytes()
                        } else {
                            Eval::check_u32(globals, *i)?.to_le_bytes()
                        };
                        Ok(bytes.to_vec().into())
                    }
                    (8, Value::Int(i)) => {
                        let bytes: &[u8] = &i.to_le_bytes();
                        Ok(bytes.to_vec().into())
                    }
                    (4, Value::Float(f)) => {
                        let bytes: &[u8] = &(*f as f32).to_le_bytes();
                        Ok(bytes.to_vec().into())
                    }
                    (8, Value::Float(f)) => {
                        let bytes: &[u8] = &f.to_le_bytes();
                        Ok(bytes.to_vec().into())
                    }
                    (_, Value::Int(_)) => globals.set_exc_str(&format!(
                        "n must be 1, 2, 4 or 8 for int val, but got {}",
                        n,
                    )),
                    (_, Value::Float(_)) => globals
                        .set_exc_str(&format!("n must be 4 or 8 for float val, but got {}", n,)),
                    (_, val) => {
                        Eval::expect_int(globals, val)?;
                        panic!("Should have returned an error")
                    }
                }
            },
        ),
        NativeFunction::sdnew(
            sr,
            "be",
            (&["n", "val"], &[], None, None),
            Some(concat!(
                "Create big endian bytes from an integer or float\n",
                "The first parameter n specifies the number of bytes to use\n",
                "The second parameter specifies the actual value to encode\n",
                "For integers, n must be one of 1, 2, 4, 8\n",
                "For floats, n must be ine of 4 or 8\n",
            )),
            |globals, args, _kwargs| {
                let n = Eval::expect_uint(globals, &args[0])?;
                match (n, &args[1]) {
                    (1, Value::Int(i)) => {
                        let bytes: &[u8] = &if *i < 0 {
                            Eval::check_i8(globals, *i)?.to_be_bytes()
                        } else {
                            Eval::check_u8(globals, *i)?.to_be_bytes()
                        };
                        Ok(bytes.to_vec().into())
                    }
                    (2, Value::Int(i)) => {
                        let bytes: &[u8] = &if *i < 0 {
                            Eval::check_i16(globals, *i)?.to_be_bytes()
                        } else {
                            Eval::check_u16(globals, *i)?.to_be_bytes()
                        };
                        Ok(bytes.to_vec().into())
                    }
                    (4, Value::Int(i)) => {
                        let bytes: &[u8] = &if *i < 0 {
                            Eval::check_i32(globals, *i)?.to_be_bytes()
                        } else {
                            Eval::check_u32(globals, *i)?.to_be_bytes()
                        };
                        Ok(bytes.to_vec().into())
                    }
                    (8, Value::Int(i)) => {
                        let bytes: &[u8] = &i.to_be_bytes();
                        Ok(bytes.to_vec().into())
                    }
                    (4, Value::Float(f)) => {
                        let bytes: &[u8] = &(*f as f32).to_be_bytes();
                        Ok(bytes.to_vec().into())
                    }
                    (8, Value::Float(f)) => {
                        let bytes: &[u8] = &f.to_be_bytes();
                        Ok(bytes.to_vec().into())
                    }
                    (_, Value::Int(_)) => globals.set_exc_str(&format!(
                        "n must be 1, 2, 4 or 8 for int val, but got {}",
                        n,
                    )),
                    (_, Value::Float(_)) => globals
                        .set_exc_str(&format!("n must be 4 or 8 for float val, but got {}", n,)),
                    (_, val) => {
                        Eval::expect_int(globals, val)?;
                        panic!("Should have returned an error")
                    }
                }
            },
        ),
    ]
    .into_iter()
    .map(|f| (sr.intern_rcstr(f.name()), Value::from(f)))
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
