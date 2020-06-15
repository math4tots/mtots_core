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

    let static_methods = vec![NativeFunction::simple0(
        sr,
        "__call",
        &["pattern"],
        |globals, args, _kwargs| {
            let bytes = Eval::expect_bytes_from_pattern(globals, &args[0])?;
            Ok(bytes.into())
        },
    )]
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
