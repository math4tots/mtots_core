use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::NativeFunction;
use crate::Symbol;
use crate::Value;

use std::rc::Rc;

pub(super) fn mkcls(base: Rc<Class>) -> Rc<Class> {
    let methods = vec![
        NativeFunction::simple0("len", &["self"], |globals, args, _kwargs| {
            let list = Eval::expect_map(globals, &args[0])?;
            Ok(Value::Int(list.len() as i64))
        }),
        NativeFunction::simple0("map", &["self", "f"], |globals, args, _kwargs| {
            let list = Eval::expect_list(globals, &args[0])?;
            let f = &args[1];
            let mut ret = Vec::new();
            for x in list {
                let y = Eval::call(globals, f, vec![x.clone()])?;
                ret.push(y);
            }
            Ok(ret.into())
        }),
        NativeFunction::simple0("filter", &["self", "f"], |globals, args, _kwargs| {
            let list = Eval::expect_list(globals, &args[0])?;
            let f = &args[1];
            let mut ret = Vec::new();
            for x in list {
                let xp = Eval::call(globals, f, vec![x.clone()])?;
                let truthy = Eval::truthy(globals, &xp)?;
                if truthy {
                    ret.push(x.clone());
                }
            }
            Ok(ret.into())
        }),
        NativeFunction::simple0("__getitem", &["self", "key"], |globals, args, _kwargs| {
            let map = Eval::expect_map(globals, &args[0])?;
            let val = map.s_get(globals, &args[1])?.cloned();
            if let Some(val) = val {
                Ok(val)
            } else {
                let keystr = Eval::repr(globals, &args[1])?;
                globals.set_key_error(
                    &format!("Key {:?} not found in given MutableMap", keystr,).into(),
                )
            }
        }),
        NativeFunction::snew(
            "get",
            (
                &["self", "key"],
                &[("default", Value::Uninitialized)],
                None,
                None,
            ),
            |globals, args, _kwargs| {
                let map = Eval::expect_map(globals, &args[0])?;
                let val = map.s_get(globals, &args[1])?.cloned();
                if let Some(val) = val {
                    Ok(val)
                } else if let Value::Uninitialized = &args[2] {
                    let keystr = Eval::repr(globals, &args[1])?;
                    globals.set_key_error(
                        &format!("Key {:?} not found in given MutableMap", keystr,).into(),
                    )
                } else {
                    Ok(args[2].clone())
                }
            },
        ),
        NativeFunction::simple0("has_key", &["self", "key"], |globals, args, _kwargs| {
            let map = Eval::expect_map(globals, &args[0])?;
            let has_key = map.s_get(globals, &args[1])?.is_some();
            Ok(has_key.into())
        }),
    ]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
    .collect();

    let static_methods = vec![
        NativeFunction::simple0("__call", &["pairs"], |globals, args, _kwargs| {
            Eval::map_from_iterable(globals, &args[0])
        }),
        NativeFunction::simple0("from_iterable", &["iterable"], |globals, args, _kwargs| {
            Eval::map_from_iterable(globals, &args[0])
        }),
    ]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
    .collect();

    Class::new0(
        ClassKind::NativeClass,
        "Map".into(),
        vec![base],
        None,
        methods,
        static_methods,
    )
    .into()
}