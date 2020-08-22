use super::*;

pub(super) fn new(iterable: &Rc<Class>) -> Rc<Class> {
    Class::new(
        "Iterator".into(),
        Class::join_class_maps(
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
                NativeFunction::new(
                    "set",
                    ["self"],
                    "Converts this iterator into a set",
                    |globals, args, _| {
                        let mut args = args.into_iter();
                        let owner = args.next().unwrap();
                        Ok(Value::from(owner.unpack_into_set(globals)?))
                    },
                ),
                NativeFunction::new(
                    "has",
                    ["self", "x"],
                    "Checks whether x is an element in this iterator\nConsumes the iterator",
                    |globals, args, _| {
                        let mut args = args.into_iter();
                        let owner = args.next().unwrap();
                        let x = args.next().unwrap();
                        Ok(contains(globals, owner, x)?.into())
                    }
                ),
                NativeFunction::new(
                    "__contains",
                    ["self", "x"],
                    "Checks whether x is an element in this iterator\nConsumes the iterator",
                    |globals, args, _| {
                        let mut args = args.into_iter();
                        let owner = args.next().unwrap();
                        let x = args.next().unwrap();
                        Ok(contains(globals, owner, x)?.into())
                    }
                ),
                NativeFunction::new(
                    "all",
                    ArgSpec::builder().req("self").def("f", ()),
                    "",
                    |globals, args, _| {
                        let mut args = args.into_iter();
                        let owner = args.next().unwrap();
                        let f = args.next().unwrap();
                        loop {
                            match owner.resume(globals, Value::Nil) {
                                ResumeResult::Yield(value) => {
                                    let cond = if f.is_nil() {
                                        value.truthy()
                                    } else {
                                        f.apply(globals, vec![value], None)?.truthy()
                                    };
                                    if !cond {
                                        return Ok(false.into());
                                    }
                                },
                                ResumeResult::Err(error) => return Err(error),
                                ResumeResult::Return(_) => return Ok(true.into()),
                            }
                        }
                    }
                ),
                NativeFunction::new(
                    "any",
                    ArgSpec::builder().req("self").def("f", ()),
                    "",
                    |globals, args, _| {
                        let mut args = args.into_iter();
                        let owner = args.next().unwrap();
                        let f = args.next().unwrap();
                        loop {
                            match owner.resume(globals, Value::Nil) {
                                ResumeResult::Yield(value) => {
                                    let cond = if f.is_nil() {
                                        value.truthy()
                                    } else {
                                        f.apply(globals, vec![value], None)?.truthy()
                                    };
                                    if cond {
                                        return Ok(true.into());
                                    }
                                },
                                ResumeResult::Err(error) => return Err(error),
                                ResumeResult::Return(_) => return Ok(false.into()),
                            }
                        }
                    }
                ),
                NativeFunction::new(
                    "enumerate",
                    ArgSpec::builder().req("self").def("start", 0),
                    "",
                    |_globals, args, _| {
                        let mut args = args.into_iter();
                        let owner = args.next().unwrap();
                        let mut i = args.next().unwrap().number()?;
                        Ok(
                            NativeGenerator::new("Iterator.enumerate", move |globals, arg| {
                                match owner.resume(globals, arg) {
                                    ResumeResult::Yield(value) => {
                                        let n = i;
                                        i += 1.0;
                                        ResumeResult::Yield(vec![Value::from(n), value].into())
                                    }
                                    r => r,
                                }
                            })
                            .into(),
                        )
                    },
                ),
                NativeFunction::new(
                    "zip",
                    ArgSpec::builder().req("self").var("others"),
                    "",
                    |globals, args, _| {
                        let iters = args
                            .into_iter()
                            .map(|v| v.iter(globals))
                            .collect::<Result<Vec<_>>>()?;
                        Ok(NativeGenerator::new("Iterator.zip", move |globals, _arg| {
                            let mut results = Vec::new();
                            for iter in &iters {
                                match iter.resume(globals, Value::Nil) {
                                    ResumeResult::Yield(x) => results.push(x),
                                    r @ ResumeResult::Err(_) => return r,
                                    ResumeResult::Return(_) => {
                                        return ResumeResult::Return(Value::Nil)
                                    }
                                }
                            }
                            ResumeResult::Yield(results.into())
                        })
                        .into())
                    },
                ),
                NativeFunction::new(
                    "filter",
                    ArgSpec::builder().req("self").def("f", ()),
                    "",
                    |_globals, args, _| {
                        let mut args = args.into_iter();
                        let owner = args.next().unwrap();
                        let f = args.next().unwrap();
                        Ok(
                            NativeGenerator::new("Iterator.filter", move |globals, _| loop {
                                match owner.resume(globals, Value::Nil) {
                                    ResumeResult::Yield(value) => {
                                        let cond = if f.is_nil() {
                                            value.truthy()
                                        } else {
                                            gentry!(f.apply(globals, vec![value.clone()], None))
                                                .truthy()
                                        };
                                        if cond {
                                            return ResumeResult::Yield(value);
                                        }
                                    }
                                    r => return r,
                                }
                            })
                            .into(),
                        )
                    },
                ),
                NativeFunction::new(
                    "map",
                    ArgSpec::builder().req("self").def("f", ()),
                    concat!(
                        "Duals as a map (in the sense of monads) and map as in, ",
                        "the data structure.\n",
                        "It is determined based on the presence of a second argument",
                    ),
                    |globals, args, _| {
                        if args[1].is_nil() {
                            // converts the iterable into a Map
                            let mut args = args.into_iter();
                            let owner = args.next().unwrap();
                            let map = owner
                                .unpack(globals)?
                                .into_iter()
                                .map(|p| p.unpack_keyval(globals))
                                .collect::<Result<Map>>()?;
                            Ok(map.into())
                        } else {
                            // creates a new iterable with 'f' applied to all arguments
                            let mut args = args.into_iter();
                            let owner = args.next().unwrap();
                            let f = args.next().unwrap();
                            Ok(NativeGenerator::new(
                                "Iterator.map",
                                move |globals, arg| match owner.resume(globals, arg) {
                                    ResumeResult::Yield(value) => ResumeResult::Yield(gentry!(
                                        f.apply(globals, vec![value], None)
                                    )),
                                    r => r,
                                },
                            )
                            .into())
                        }
                    },
                ),
            ]),
            vec![iterable],
        ),
        HashMap::new(),
    )
}

fn contains(globals: &mut Globals, owner: Value, item: Value) -> Result<bool> {
    loop {
        match owner.resume(globals, Value::Nil) {
            ResumeResult::Yield(value) => if item == value { return Ok(true) },
            ResumeResult::Err(error) => return Err(error),
            ResumeResult::Return(_) => return Ok(false),
        }
    }
}
