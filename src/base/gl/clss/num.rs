use super::*;

pub(super) fn new() -> Rc<Class> {
    Class::new(
        "Number".into(),
        Class::map_from_funcs(vec![NativeFunction::new(
            "fract",
            ["self"],
            "",
            |_globals, args, _| {
                let mut args = args.into_iter();
                let x = args.next().unwrap().f64()?.fract();
                Ok(x.into())
            },
        )]),
        HashMap::new(),
    )
}
