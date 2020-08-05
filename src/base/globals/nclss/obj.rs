use crate::Class;
use crate::ClassKind;

use std::collections::HashMap;
use std::rc::Rc;

pub(super) fn mkcls() -> Rc<Class> {
    Class::new0(
        ClassKind::Trait,
        "Object".into(),
        vec![],
        Some(concat!(
            "Trait Object is the root of the trait hierarchy\n",
            "All classes should either directly or indirectly inherit from this trait\n",
        )),
        HashMap::new(),
        HashMap::new(),
    )
    .into()
}
