use super::*;

pub(super) fn new(iterable: &Rc<Class>) -> Rc<Class> {
    Class::new(
        "Map".into(),
        Class::join_class_maps(
            Class::map_from_funcs(vec![NativeFunction::new(
                "len",
                ["self"],
                "",
                |_globals, args, _| {
                    let mut args = args.into_iter();
                    let owner = args.next().unwrap().into_map()?;
                    let owner = owner.borrow();
                    Ok(owner.len().into())
                },
            )]),
            vec![iterable],
        ),
        Class::map_from_funcs(vec![NativeFunction::new(
            "__from_iterable",
            ["iterable"],
            "",
            |globals, args, _| {
                let mut args = args.into_iter();
                let iterable = args.next().unwrap();
                map_from_iterable(globals, iterable)
            },
        )]),
    )
}

fn map_from_iterable(globals: &mut Globals, iter: Value) -> Result<Value> {
    match iter {
        Value::Map(map) => Ok(Value::from(Map::unwrap_or_clone(map))),
        _ => Ok(iter
            .unpack(globals)?
            .into_iter()
            .map(|pair| pair.unpack_keyval(globals))
            .collect::<Result<Map>>()?
            .into()),
    }
}
