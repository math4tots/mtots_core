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
            let set = Eval::expect_mutable_set(globals, &args[0])?;
            Ok(Value::Int(set.borrow().len() as i64))
        }),
        NativeFunction::new("has", &["self", "key"], None, |globals, args, _kwargs| {
            let set = Eval::expect_mutable_set(globals, &args[0])?;
            Ok(set.borrow().s_get(globals, &args[1])?.is_some().into())
        }),
        NativeFunction::new("add", &["self", "key"], None, |globals, args, _kwargs| {
            let set = Eval::expect_mutable_set(globals, &args[0])?;
            set.borrow_mut().s_insert(globals, args[1].clone(), ())?;
            Ok(Value::Nil)
        }),
        NativeFunction::new("move", &["self"], None, |globals, args, _kwargs| {
            let set = Eval::expect_mutable_set(globals, &args[0])?;
            let vset = set.replace(VSet::new());
            Ok(vset.into())
        }),
    ]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
    .collect();

    let static_methods = vec![
        NativeFunction::new("__call", &["iterable"], None, |globals, args, _kwargs| {
            Eval::mutable_set_from_iterable(globals, &args[0])
        }),
        NativeFunction::new(
            "from_iterable",
            &["iterable"],
            None,
            |globals, args, _kwargs| Eval::mutable_set_from_iterable(globals, &args[0]),
        ),
    ]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
    .collect();

    Class::new0(
        ClassKind::NativeClass,
        "MutableSet".into(),
        vec![base],
        None,
        methods,
        static_methods,
    )
    .into()
}
