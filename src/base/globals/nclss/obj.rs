use crate::Class;
use crate::ClassKind;
use crate::SymbolRegistryHandle;

use std::collections::HashMap;
use std::rc::Rc;

pub(super) fn mkcls(_symbol_registry: &SymbolRegistryHandle) -> Rc<Class> {
    Class::new0(
        ClassKind::Trait,
        "Object".into(),
        vec![],
        HashMap::new(),
        HashMap::new(),
    )
    .into()
}
