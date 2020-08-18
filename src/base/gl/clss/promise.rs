use super::*;

pub(super) fn new() -> Rc<Class> {
    Class::new(
        "Promise".into(),
        Class::map_from_funcs(vec![NativeFunction::new(
            "ordie",
            ["self"],
            "",
            |globals, args, _| {
                let mut args = args.into_iter();
                let owner = args.next().unwrap().into_promise()?;
                owner.borrow_mut().ordie(globals);
                Ok(Value::Nil)
            },
        )]),
        HashMap::new(),
    )
}
