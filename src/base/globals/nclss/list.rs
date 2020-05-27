use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::NativeFunction;
use crate::SymbolRegistryHandle;
use crate::Value;

use std::rc::Rc;

pub(super) fn mkcls(sr: &SymbolRegistryHandle, base: Rc<Class>) -> Rc<Class> {
    let methods = vec![
        NativeFunction::simple0(sr, "len", &["self"], |globals, args, _kwargs| {
            let list = Eval::expect_list(globals, &args[0])?;
            Ok(Value::Int(list.len() as i64))
        }),
        NativeFunction::simple0(sr, "__getitem", &["self", "i"], |globals, args, _kwargs| {
            let list = Eval::expect_list(globals, &args[0])?;
            let i = Eval::expect_index(globals, &args[1], list.len())?;
            Ok(list[i].clone())
        }),
        NativeFunction::simple0(sr, "map", &["self", "f"], |globals, args, _kwargs| {
            let list = Eval::expect_list(globals, &args[0])?;
            let f = &args[1];
            let mut ret = Vec::new();
            for x in list {
                let y = Eval::call(globals, f, vec![x.clone()])?;
                ret.push(y);
            }
            Ok(ret.into())
        }),
        NativeFunction::simple0(sr, "filter", &["self", "f"], |globals, args, _kwargs| {
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
        NativeFunction::simple0(sr, "iter", &["self"], |globals, args, _kwargs| {
            Eval::iter(globals, &args[0])
        }),
    ]
    .into_iter()
    .map(|f| (sr.intern_rcstr(f.name()), Value::from(f)))
    .collect();

    let static_methods = vec![
        NativeFunction::simple0(sr, "__call", &["iterable"], |globals, args, _kwargs| {
            Eval::list_from_iterable(globals, &args[0])
        }),
        NativeFunction::simple0(
            sr,
            "from_iterable",
            &["iterable"],
            |globals, args, _kwargs| Eval::list_from_iterable(globals, &args[0]),
        ),
    ]
    .into_iter()
    .map(|f| (sr.intern_rcstr(f.name()), Value::from(f)))
    .collect();

    Class::new0(
        ClassKind::NativeClass,
        "List".into(),
        vec![base],
        methods,
        static_methods,
    )
    .into()
}