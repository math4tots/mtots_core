use crate::Class;
use crate::ClassKind;
use crate::SymbolRegistryHandle;

use std::collections::HashMap;
use std::rc::Rc;

pub(super) fn mkcls(_symbol_registry: &SymbolRegistryHandle, base: Rc<Class>) -> Rc<Class> {
    Class::new0(
        ClassKind::NativeClass,
        "Function".into(),
        vec![base],
        None,
        HashMap::new(),
        HashMap::new(),
    )
    .into()
}
