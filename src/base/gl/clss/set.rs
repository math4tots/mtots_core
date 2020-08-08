use super::*;

pub(super) fn new(iterable: &Rc<Class>) -> Rc<Class> {
    Class::new(
        "Set".into(),
        Class::join_class_maps(
            Class::map_from_funcs(vec![
                NativeFunction::new("len", ["self"], None, |_globals, args, _| {
                    let mut args = args.into_iter();
                    let owner = args.next().unwrap().into_set()?;
                    let owner = owner.borrow();
                    Ok(owner.len().into())
                }),
                NativeFunction::new("add", ["self", "x"], None, |_globals, args, _| {
                    let mut args = args.into_iter();
                    let owner = args.next().unwrap().into_set()?;
                    let mut owner = owner.borrow_mut();
                    let key = Key::try_from(args.next().unwrap())?;
                    owner.insert(key);
                    Ok(Value::Nil)
                }),
                NativeFunction::new("has", ["self", "x"], None, |_globals, args, _| {
                    let mut args = args.into_iter();
                    let owner = args.next().unwrap().into_set()?;
                    let owner = owner.borrow();
                    let key = Key::try_from(args.next().unwrap())?;
                    Ok(owner.contains(&key).into())
                }),
                NativeFunction::new("__add", ["self", "other"], None, |_globals, args, _| {
                    let mut args = args.into_iter();
                    let mut owner = Set::unwrap_or_clone(args.next().unwrap().into_set()?);
                    let other = args.next().unwrap().into_set()?;
                    owner.extend(other.borrow().iter().map(Clone::clone));
                    Ok(owner.into())
                }),
            ]),
            vec![iterable],
        ),
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
