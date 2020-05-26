use crate::Class;
use crate::ClassKind;
use crate::SymbolRegistryHandle;

use std::collections::HashMap;
use std::rc::Rc;

pub(super) fn mkcls(_: &SymbolRegistryHandle, base: Rc<Class>) -> Rc<Class> {
    let methods = HashMap::new();
    let static_methods = HashMap::new();

    Class::new0(
        ClassKind::NativeClass,
        "Opaque".into(),
        vec![base],
        methods,
        static_methods,
    )
    .into()
}
