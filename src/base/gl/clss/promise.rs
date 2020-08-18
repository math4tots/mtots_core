use super::*;

pub(super) fn new() -> Rc<Class> {
    Class::new(
        "Promise".into(),
        Class::map_from_funcs(vec![]),
        HashMap::new(),
    )
}
