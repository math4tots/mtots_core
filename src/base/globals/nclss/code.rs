use crate::Class;
use crate::ClassKind;

use std::collections::HashMap;
use std::rc::Rc;

pub(super) fn mkcls(base: Rc<Class>) -> Rc<Class> {
    Class::new0(
        ClassKind::NativeClass,
        "Code".into(),
        vec![base],
        None,
        HashMap::new(),
        HashMap::new(),
    )
    .into()
}
