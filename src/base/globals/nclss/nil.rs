use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::NativeFunction;
use crate::SymbolRegistryHandle;
use crate::Value;
use std::collections::HashMap;
use alloc::rc::Rc;

pub(super) fn mkcls(sr: &SymbolRegistryHandle, base: Rc<Class>) -> Rc<Class> {
    let methods = vec![
        NativeFunction::simple0(sr, "map", &["self", "x", "f"], |globals, args, _kwargs| {
            // Returns nil if x is nil, otherwise f(x)
            if let Value::Nil = &args[1] {
                Ok(Value::Nil)
            } else {
                Eval::call(globals, &args[2], vec![args[1].clone()])
            }
        }),
        NativeFunction::simple0(
            sr,
            "vmap",
            &["self", "x", "y"],
            |_globals, args, _kwargs| {
                // Returns nil if x is nil, otherwise y
                if let Value::Nil = &args[1] {
                    Ok(Value::Nil)
                } else {
                    Ok(args[2].clone())
                }
            },
        ),
        NativeFunction::simple0(sr, "get", &["self", "x", "y"], |_globals, args, _kwargs| {
            // Returns x if x is not nil, and y if x is nil
            if let Value::Nil = &args[1] {
                Ok(args[2].clone())
            } else {
                Ok(args[1].clone())
            }
        }),
        NativeFunction::simple0(sr, "fget", &["self", "x", "f"], |globals, args, _kwargs| {
            // Returns x if x is not nil, and f() if x is nil
            if let Value::Nil = &args[1] {
                Eval::call(globals, &args[2], vec![])
            } else {
                Ok(args[1].clone())
            }
        }),
    ]
    .into_iter()
    .map(|f| (sr.intern_rcstr(f.name()), Value::from(f)))
    .collect();

    Class::new0(
        ClassKind::NativeClass,
        "Nil".into(),
        vec![base],
        None,
        methods,
        HashMap::new(),
    )
    .into()
}
