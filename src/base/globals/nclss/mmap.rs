use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::NativeFunction;
use crate::SymbolRegistryHandle;
use crate::VMap;
use crate::Value;
use std::rc::Rc;

pub(super) fn mkcls(sr: &SymbolRegistryHandle, base: Rc<Class>) -> Rc<Class> {
    let methods = vec![
        NativeFunction::simple0(sr, "move", &["self"], |globals, args, _kwargs| {
            let map = Eval::expect_mutable_map(globals, &args[0])?;
            let map = map.replace(VMap::new());
            Ok(Value::Map(map.into()))
        }),
        NativeFunction::simple0(sr, "len", &["self"], |globals, args, _kwargs| {
            let map = Eval::expect_mutable_map(globals, &args[0])?;
            Ok(Value::Int(map.borrow().len() as i64))
        }),
        NativeFunction::simple0(
            sr,
            "__getitem",
            &["self", "key"],
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
        NativeFunction::simple0(
            sr,
            "__setitem",
            &["self", "key", "val"],
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
        NativeFunction::simple0(sr, "has_key", &["self", "key"], |globals, args, _kwargs| {
            let map = Eval::expect_mutable_map(globals, &args[0])?;
            let has_key = map.borrow().s_get(globals, &args[1])?.is_some();
            Ok(has_key.into())
        }),
    ]
    .into_iter()
    .map(|f| (sr.intern_rcstr(f.name()), Value::from(f)))
    .collect();

    let static_methods = vec![NativeFunction::simple0(
        sr,
        "new",
        &["__call"],
        |globals, args, _kwargs| Eval::mutable_map_from_iterable(globals, &args[0]),
    )]
    .into_iter()
    .map(|f| (sr.intern_rcstr(f.name()), Value::from(f)))
    .collect();

    Class::new0(
        ClassKind::NativeClass,
        "MutableMap".into(),
        vec![base],
        methods,
        static_methods,
    )
    .into()
}
