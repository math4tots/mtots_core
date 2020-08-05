use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::NativeFunction;
use crate::ParameterInfo;
use crate::Symbol;
use crate::VMap;
use crate::Value;
use std::rc::Rc;

pub(super) fn mkcls(base: Rc<Class>) -> Rc<Class> {
    let methods = vec![
        NativeFunction::new("move", &["self"], None, |globals, args, _kwargs| {
            let map = Eval::expect_mutable_map(globals, &args[0])?;
            let map = map.replace(VMap::new());
            Ok(Value::Map(map.into()))
        }),
        NativeFunction::new("len", &["self"], None, |globals, args, _kwargs| {
            let map = Eval::expect_mutable_map(globals, &args[0])?;
            Ok(Value::from(map.borrow().len() as i64))
        }),
        NativeFunction::new(
            "__getitem",
            &["self", "key"],
            None,
            |globals, args, _kwargs| {
                let map = Eval::expect_mutable_map(globals, &args[0])?;
                let val = map.borrow().s_get(globals, &args[1])?.cloned();
                if let Some(val) = val {
                    Ok(val)
                } else {
                    let keystr = Eval::repr(globals, &args[1])?;
                    globals.set_key_error(
                        &format!("Key {:?} not found in given MutableMap", keystr,).into(),
                    )
                }
            },
        ),
        NativeFunction::new(
            "get",
            ParameterInfo::builder()
                .required("self")
                .required("key")
                .optional("default", Value::Uninitialized),
            None,
            |globals, args, _kwargs| {
                let map = Eval::expect_mutable_map(globals, &args[0])?;
                let val = map.borrow().s_get(globals, &args[1])?.cloned();
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
        NativeFunction::new(
            "__setitem",
            &["self", "key", "val"],
            None,
            |globals, args, _kwargs| {
                let mut args = args.into_iter();
                let map = args.next().unwrap();
                let map = Eval::expect_mutable_map(globals, &map)?;
                let key = args.next().unwrap();
                let val = args.next().unwrap();
                map.borrow_mut().s_insert(globals, key, val.clone())?;
                Ok(Value::Nil)
            },
        ),
        NativeFunction::new(
            "has_key",
            &["self", "key"],
            None,
            |globals, args, _kwargs| {
                let map = Eval::expect_mutable_map(globals, &args[0])?;
                let has_key = map.borrow().s_get(globals, &args[1])?.is_some();
                Ok(has_key.into())
            },
        ),
    ]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
    .collect();

    let static_methods = vec![NativeFunction::new(
        "__call",
        &["x"],
        None,
        |globals, args, _kwargs| Eval::mutable_map_from_iterable(globals, &args[0]),
    )]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
    .collect();

    Class::new0(
        ClassKind::NativeClass,
        "MutableMap".into(),
        vec![base],
        None,
        methods,
        static_methods,
    )
    .into()
}
