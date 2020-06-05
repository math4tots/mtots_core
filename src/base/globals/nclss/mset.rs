use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::NativeFunction;
use crate::SymbolRegistryHandle;
use crate::VSet;
use crate::Value;
use std::rc::Rc;

pub(super) fn mkcls(sr: &SymbolRegistryHandle, base: Rc<Class>) -> Rc<Class> {
    let methods = vec![
        NativeFunction::simple0(sr, "len", &["self"], |globals, args, _kwargs| {
            let set = Eval::expect_mutable_set(globals, &args[0])?;
            Ok(Value::Int(set.borrow().len() as i64))
        }),
        NativeFunction::simple0(sr, "has", &["self", "key"], |globals, args, _kwargs| {
            let set = Eval::expect_mutable_set(globals, &args[0])?;
            Ok(set.borrow().s_get(globals, &args[1])?.is_some().into())
        }),
        NativeFunction::simple0(sr, "add", &["self", "key"], |globals, args, _kwargs| {
            let set = Eval::expect_mutable_set(globals, &args[0])?;
            set.borrow_mut().s_insert(globals, args[1].clone(), ())?;
            Ok(Value::Nil)
        }),
        NativeFunction::simple0(sr, "move", &["self"], |globals, args, _kwargs| {
            let set = Eval::expect_mutable_set(globals, &args[0])?;
            let vset = set.replace(VSet::new());
            Ok(vset.into())
        }),
    ]
    .into_iter()
    .map(|f| (sr.intern_rcstr(f.name()), Value::from(f)))
    .collect();

    let static_methods = vec![
        NativeFunction::simple0(sr, "__call", &["iterable"], |globals, args, _kwargs| {
            Eval::mutable_set_from_iterable(globals, &args[0])
        }),
        NativeFunction::simple0(
            sr,
            "from_iterable",
            &["iterable"],
            |globals, args, _kwargs| Eval::mutable_set_from_iterable(globals, &args[0]),
        ),
    ]
    .into_iter()
    .map(|f| (sr.intern_rcstr(f.name()), Value::from(f)))
    .collect();

    Class::new0(
        ClassKind::NativeClass,
        "MutableSet".into(),
        vec![base],
        methods,
        static_methods,
    )
    .into()
}
