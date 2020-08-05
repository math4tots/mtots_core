use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::NativeFunction;
use crate::Symbol;
use crate::VSet;
use crate::Value;
use std::rc::Rc;

pub(super) fn mkcls(base: Rc<Class>) -> Rc<Class> {
    let methods = vec![
        NativeFunction::new("len", &["self"], None, |globals, args, _kwargs| {
            let set = Eval::expect_set(globals, &args[0])?;
            Ok(Value::from(set.len() as f64))
        }),
        NativeFunction::new("has", &["self", "key"], None, |globals, args, _kwargs| {
            let set = Eval::expect_set(globals, &args[0])?;
            Ok(set.s_get(globals, &args[1])?.is_some().into())
        }),
        NativeFunction::new("map", &["self", "f"], None, |globals, args, _kwargs| {
            let set = Eval::expect_set(globals, &args[0])?;
            let f = &args[1];
            let mut ret = VSet::new();
            for (x, ()) in set {
                let y = Eval::call(globals, f, vec![x.clone()])?;
                ret.s_insert(globals, y, ())?;
            }
            Ok(ret.into())
        }),
        NativeFunction::new("filter", &["self", "f"], None, |globals, args, _kwargs| {
            let set = Eval::expect_set(globals, &args[0])?;
            let f = &args[1];
            let mut ret = VSet::new();
            for (x, ()) in set {
                let xp = Eval::call(globals, f, vec![x.clone()])?;
                let truthy = Eval::truthy(globals, &xp)?;
                if truthy {
                    ret.s_insert(globals, x.clone(), ())?;
                }
            }
            Ok(ret.into())
        }),
    ]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
    .collect();

    let static_methods = vec![
        NativeFunction::new("__call", &["iterable"], None, |globals, args, _kwargs| {
            Eval::set_from_iterable(globals, &args[0])
        }),
        NativeFunction::new(
            "from_iterable",
            &["iterable"],
            None,
            |globals, args, _kwargs| Eval::set_from_iterable(globals, &args[0]),
        ),
    ]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
    .collect();

    Class::new0(
        ClassKind::NativeClass,
        "Set".into(),
        vec![base],
        None,
        methods,
        static_methods,
    )
    .into()
}
