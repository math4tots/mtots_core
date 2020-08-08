use super::*;

pub(super) fn new(iterable: &Rc<Class>) -> Rc<Class> {
    Class::new(
        "List".into(),
        Class::join_class_maps(
            Class::map_from_funcs(vec![
                NativeFunction::new("push", ["self", "x"], None, |_globals, args, _| {
                    let mut args = args.into_iter();
                    let owner = args.next().unwrap().into_list()?;
                    let x = args.next().unwrap();
                    owner.borrow_mut().push(x);
                    Ok(Value::Nil)
                }),
                NativeFunction::new("pop", ["self"], None, |_globals, args, _| {
                    let mut args = args.into_iter();
                    let owner = args.next().unwrap().into_list()?;
                    let r = owner.borrow_mut().pop();
                    if let Some(x) = r {
                        Ok(x)
                    } else {
                        Err(rterr!("Pop from empty list"))
                    }
                }),
                NativeFunction::new("map", ["self", "f"], None, |globals, args, _| {
                    let mut args = args.into_iter();
                    let owner = args.next().unwrap().into_list()?;
                    let f = args.next().unwrap();
                    let mut ret = Vec::<Value>::new();
                    for x in owner.borrow().iter() {
                        ret.push(f.apply(globals, vec![x.clone()], None)?);
                    }
                    Ok(ret.into())
                }),
                NativeFunction::new("filter", ["self", "f"], None, |globals, args, _| {
                    let mut args = args.into_iter();
                    let owner = args.next().unwrap().into_list()?;
                    let f = args.next().unwrap();
                    let mut ret = Vec::<Value>::new();
                    for x in owner.borrow().iter() {
                        if f.apply(globals, vec![x.clone()], None)?.truthy() {
                            ret.push(x.clone());
                        }
                    }
                    Ok(ret.into())
                }),
                NativeFunction::new("has", ["self", "x"], None, |_globals, args, _| {
                    let mut args = args.into_iter();
                    let owner = args.next().unwrap().into_list()?;
                    let x = args.next().unwrap();
                    for value in owner.borrow().iter() {
                        if &x == value {
                            return Ok(true.into());
                        }
                    }
                    Ok(false.into())
                }),
            ]),
            vec![iterable],
        ),
        Class::map_from_funcs(vec![NativeFunction::new(
            "__from_iterable",
            ["iterable"],
            "",
            |globals, args, _| {
                let iter = args.into_iter().next().unwrap();
                match iter {
                    Value::List(list) => Ok(Value::from(List::unwrap_or_clone(list))),
                    _ => Ok(Value::from(iter.unpack(globals)?)),
                }
            },
        )]),
    )
}
