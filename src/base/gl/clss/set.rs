use super::*;

pub(super) fn new(iterable: &Rc<Class>) -> Rc<Class> {
    Class::new(
        "Set".into(),
        Class::join_class_maps(HashMap::new(), vec![iterable]),
        Class::map_from_funcs(vec![NativeFunction::new(
            "__call",
            ["x"],
            None,
            |globals, args, _| {
                let x = args.into_iter().next().unwrap();
                let set = x.unpack_into_set(globals)?;
                Ok(set.into())
            },
        )]),
    )
}
