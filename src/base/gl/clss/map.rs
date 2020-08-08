use super::*;

pub(super) fn new(iterable: &Rc<Class>) -> Rc<Class> {
    Class::new(
        "Map".into(),
        Class::join_class_maps(HashMap::new(), vec![iterable]),
        HashMap::new(),
    )
}
