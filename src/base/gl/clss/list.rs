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
                NativeFunction::new("insert", ["self", "i", "x"], None, |_globals, args, _| {
                    let mut args = args.into_iter();
                    let owner = args.next().unwrap().into_list()?;
                    let mut owner = owner.borrow_mut();
                    let i = args.next().unwrap().to_index(owner.len())?;
                    let x = args.next().unwrap();
                    owner.insert(i, x);
                    Ok(Value::Nil)
                }),
                NativeFunction::new("remove", ["self", "i"], None, |_globals, args, _| {
                    let mut args = args.into_iter();
                    let owner = args.next().unwrap().into_list()?;
                    let mut owner = owner.borrow_mut();
                    let i = args.next().unwrap().to_index(owner.len())?;
                    Ok(owner.remove(i))
                }),
                NativeFunction::new("__mul", ["self", "n"], "", |_globals, args, _| {
                    let mut args = args.into_iter();
                    let owner = args.next().unwrap().into_list()?;
                    let owner = owner.borrow();
                    let n = usize::try_from(args.next().unwrap())?;
                    let mut ret = Vec::new();
                    for _ in 0..n {
                        ret.extend(owner.iter().map(Clone::clone));
                    }
                    Ok(ret.into())
                }),
                NativeFunction::new(
                    "all",
                    ArgSpec::builder().req("self").def("f", ()),
                    None,
                    |globals, args, _| {
                        let mut args = args.into_iter();
                        let owner = args.next().unwrap().into_list()?;
                        let f = args.next().unwrap();
                        for x in owner.borrow().iter() {
                            let cond = if f.is_nil() {
                                x.truthy()
                            } else {
                                f.apply(globals, vec![x.clone()], None)?.truthy()
                            };
                            if !cond {
                                return Ok(false.into());
                            }
                        }
                        Ok(true.into())
                    },
                ),
                NativeFunction::new(
                    "any",
                    ArgSpec::builder().req("self").def("f", ()),
                    None,
                    |globals, args, _| {
                        let mut args = args.into_iter();
                        let owner = args.next().unwrap().into_list()?;
                        let f = args.next().unwrap();
                        for x in owner.borrow().iter() {
                            let cond = if f.is_nil() {
                                x.truthy()
                            } else {
                                f.apply(globals, vec![x.clone()], None)?.truthy()
                            };
                            if cond {
                                return Ok(true.into());
                            }
                        }
                        Ok(false.into())
                    },
                ),
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
        Class::map_from_funcs(vec![
            NativeFunction::new("__from_iterable", ["iterable"], "", |globals, args, _| {
                let iter = args.into_iter().next().unwrap();
                list_from_iterable(globals, iter)
            }),
            NativeFunction::new("__call", ["iterable"], "", |globals, args, _| {
                let iter = args.into_iter().next().unwrap();
                list_from_iterable(globals, iter)
            }),
        ]),
    )
}

fn list_from_iterable(globals: &mut Globals, iter: Value) -> Result<Value> {
    match iter {
        Value::List(list) => Ok(Value::from(List::unwrap_or_clone(list))),
        _ => Ok(Value::from(iter.unpack(globals)?)),
    }
}
