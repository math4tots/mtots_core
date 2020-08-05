use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::NativeFunction;
use crate::Symbol;
use crate::Value;
use std::collections::HashMap;
use std::rc::Rc;

pub(super) fn mkcls(base: Rc<Class>) -> Rc<Class> {
    let methods = vec![
        NativeFunction::new(
            "map",
            &["self", "x", "f"],
            None,
            |globals, args, _kwargs| {
                // Returns nil if x is nil, otherwise f(x)
                if let Value::Nil = &args[1] {
                    Ok(Value::Nil)
                } else {
                    Eval::call(globals, &args[2], vec![args[1].clone()])
                }
            },
        ),
        NativeFunction::new(
            "vmap",
            &["self", "x", "y"],
            None,
            |_globals, args, _kwargs| {
                // Returns nil if x is nil, otherwise y
                if let Value::Nil = &args[1] {
                    Ok(Value::Nil)
                } else {
                    Ok(args[2].clone())
                }
            },
        ),
        NativeFunction::new(
            "get",
            &["self", "x", "y"],
            None,
            |_globals, args, _kwargs| {
                // Returns x if x is not nil, and y if x is nil
                if let Value::Nil = &args[1] {
                    Ok(args[2].clone())
                } else {
                    Ok(args[1].clone())
                }
            },
        ),
        NativeFunction::new(
            "fget",
            &["self", "x", "f"],
            None,
            |globals, args, _kwargs| {
                // Returns x if x is not nil, and f() if x is nil
                if let Value::Nil = &args[1] {
                    Eval::call(globals, &args[2], vec![])
                } else {
                    Ok(args[1].clone())
                }
            },
        ),
    ]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
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
