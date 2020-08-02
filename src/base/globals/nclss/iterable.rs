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
        NativeFunction::simple0("to", &["self", "type"], |globals, args, _kwargs| {
            let iterable = &args[0];
            let type_ = &args[1];
            Eval::from_iterable(globals, type_, iterable.clone())
        }),
        NativeFunction::simple0("iter", &["self"], |globals, args, _kwargs| {
            // Even though I want this to be the default way to get iterators,
            // I think to implement an iterable, a class should have a `__iter` method
            Eval::iter(globals, &args[0])
        }),
    ]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
    .collect();

    let static_methods = HashMap::new();

    Class::new0(
        ClassKind::Trait,
        "Iterable".into(),
        vec![base],
        None,
        methods,
        static_methods,
    )
    .into()
}
