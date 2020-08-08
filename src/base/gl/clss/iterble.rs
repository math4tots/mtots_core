use super::*;

pub(super) fn new() -> Rc<Class> {
    Class::new(
        "Iterable".into(),
        Class::map_from_funcs(vec![NativeFunction::new(
            "iter",
            ["self"],
            "",
            |globals, args, _| {
                let owner = args.into_iter().next().unwrap();
                owner.iter(globals)
            },
        )]),
        HashMap::new(),
    )
}
