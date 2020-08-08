use super::*;

pub(super) fn new() -> Rc<Class> {
    Class::new(
        "Iterator".into(),
        Class::map_from_funcs(vec![
            NativeFunction::new(
                "to",
                ["self", "type"],
                "Converts this iterator into the target collection type",
                |globals, args, _| {
                    let mut args = args.into_iter();
                    let owner = args.next().unwrap();
                    let type_ = args.next().unwrap();
                    type_.apply_method(globals, "__from_iterable", vec![owner], None)
                },
            ),
            NativeFunction::new(
                "list",
                ["self"],
                "Converts this iterator into a list",
                |globals, args, _| {
                    let mut args = args.into_iter();
                    let owner = args.next().unwrap();
                    Ok(Value::from(owner.unpack(globals)?))
                },
            ),
        ]),
        HashMap::new(),
    )
}
