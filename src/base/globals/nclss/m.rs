use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::NativeFunction;
use crate::SymbolRegistryHandle;
use crate::Value;

use std::collections::HashMap;
use std::rc::Rc;

pub(super) fn mkcls(sr: &SymbolRegistryHandle, base: Rc<Class>) -> Rc<Class> {
    let methods = vec![
        NativeFunction::simple0(sr, "keys", &["self"], |globals, args, _kwargs| {
            let m = Eval::expect_module(globals, &args[0])?;
            let keys: Vec<_> = m.keys().map(|s| Value::from(*s)).collect();
            Ok(keys.into())
        }),
        NativeFunction::simple0(sr, "get", &["self", "key"], |globals, args, _kwargs| {
            let key = Eval::expect_symbollike(globals, &args[1])?;
            Eval::get_static_attr_or_err(globals, &args[0], key)
        }),
    ]
    .into_iter()
    .map(|f| (sr.intern_rcstr(f.name()), Value::from(f)))
    .collect();

    let static_methods = HashMap::new();

    Class::new0(
        ClassKind::NativeClass,
        "Module".into(),
        vec![base],
        methods,
        static_methods,
    )
    .into()
}
