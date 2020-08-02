use crate::Class;
use crate::ClassKind;

use std::collections::HashMap;
use std::rc::Rc;

pub(super) fn mkcls(base: Rc<Class>) -> Rc<Class> {
    let methods = HashMap::new();
    let static_methods = HashMap::new();

    Class::new0(
        ClassKind::NativeClass,
        "Opaque".into(),
        vec![base],
        None,
        methods,
        static_methods,
    )
    .into()
}
