use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::NativeFunction;
use crate::SymbolRegistryHandle;
use crate::Value;
use std::collections::HashMap;
use std::rc::Rc;

pub(super) fn mkcls(sr: &SymbolRegistryHandle, base: Rc<Class>) -> Rc<Class> {
    let static_methods = vec![
        NativeFunction::simple0(sr, "__call", &["symbollike"], |globals, args, _kwargs| {
            Ok(Eval::expect_symbollike(globals, &args[0])?.into())
        }),
    ]
    .into_iter()
    .map(|f| (sr.intern_rcstr(f.name()), Value::from(f)))
    .collect();

    Class::new0(
        ClassKind::NativeClass,
        "Symbol".into(),
        vec![base],
        None,
        HashMap::new(),
        static_methods,
    )
    .into()
}
