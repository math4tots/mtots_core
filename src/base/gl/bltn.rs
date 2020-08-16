use super::*;

impl Globals {
    pub(super) fn bootstrap_new_builtins(class_manager: &ClassManager) -> HashMap<RcStr, Value> {
        let mut map = HashMap::new();
        for cls in class_manager.list() {
            map.insert(cls.name().clone(), cls.clone().into());
        }

        let funcs = vec![
            NativeFunction::new("print", ["x"], None, |globals, args, _| {
                let x = args.into_iter().next().unwrap();
                globals.print(&format!("{}\n", x));
                Ok(Value::Nil)
            }),
            NativeFunction::new("str", ["x"], None, |_globals, args, _| {
                Ok(args.into_iter().next().unwrap().convert_to_rcstr().into())
            }),
            NativeFunction::new("repr", ["x"], None, |_globals, args, _| {
                Ok(format!("{:?}", args.into_iter().next().unwrap()).into())
            }),
            NativeFunction::new("chr", ["x"], None, |_globals, args, _| {
                let i = u32::try_from(args.into_iter().next().unwrap())?;
                let c = char::try_from(i)?;
                Ok(Value::from(c))
            }),
            NativeFunction::new("ord", ["c"], None, |_globals, args, _| {
                let s = args.into_iter().next().unwrap().into_string()?;
                let nchars = s.chars().count();
                if nchars != 1 {
                    Err(rterr!(
                        "ord requires a string of length 1, but got {:?} (len = {})",
                        s,
                        nchars,
                    ))
                } else {
                    let c = s.chars().next().unwrap();
                    Ok(Value::from(c as u32))
                }
            }),
            NativeFunction::new("reversed", ["iterable"], None, |globals, args, _| {
                let mut vec = args.into_iter().next().unwrap().unpack(globals)?;
                vec.reverse();
                Ok(vec.into())
            }),
            NativeFunction::new("sorted", ["iterable"], None, |globals, args, _| {
                let mut vec = args.into_iter().next().unwrap().unpack(globals)?;
                List::sort(&mut vec)?;
                Ok(vec.into())
            }),
            NativeFunction::new(
                "range",
                ArgSpec::builder()
                    .req("start")
                    .def("stop", ())
                    .def("step", 1),
                None,
                |_globals, args, _| {
                    let args1_is_nil = args[1].is_nil();
                    let mut args = args.into_iter();
                    let (mut start, stop) = if args1_is_nil {
                        let stop = args.next().unwrap().number()?;
                        args.next().unwrap();
                        (0.0, stop)
                    } else {
                        let start = args.next().unwrap().number()?;
                        let stop = args.next().unwrap().number()?;
                        (start, stop)
                    };
                    let step = args.next().unwrap().number()?;
                    if step > 0.0 {
                        Ok(NativeGenerator::new("range", move |_globals, _| {
                            if start < stop {
                                let x = start;
                                start += step;
                                ResumeResult::Yield(Value::from(x))
                            } else {
                                ResumeResult::Return(Value::Nil)
                            }
                        })
                        .into())
                    } else if step < 0.0 {
                        Ok(NativeGenerator::new("range", move |_globals, _| {
                            if start > stop {
                                let x = start;
                                start += step;
                                ResumeResult::Yield(Value::from(x))
                            } else {
                                ResumeResult::Return(Value::Nil)
                            }
                        })
                        .into())
                    } else {
                        Err(rterr!("Invalid range step: {}", step))
                    }
                },
            ),
            NativeFunction::new("sorted", ["x"], None, |globals, args, _| {
                let mut list = args.into_iter().next().unwrap().unpack(globals)?;
                list.sort_by(|a, b| a.partial_cmp(&b).unwrap_or(cmp::Ordering::Equal));
                Ok(list.into())
            }),
            NativeFunction::new("assert_eq", ["a", "b"], None, |_globals, args, _| {
                let mut args = args.into_iter();
                let a = args.next().unwrap();
                let b = args.next().unwrap();
                if a != b {
                    Err(rterr!("Expected {:?} to equal {:?}", a, b))
                } else {
                    Ok(Value::Nil)
                }
            }),
            NativeFunction::new("assert_throws", ["f"], None, |globals, args, _| {
                let mut args = args.into_iter();
                let f = args.next().unwrap();
                let trace_len = globals.trace().len();
                match f.apply(globals, vec![], None) {
                    Ok(_) => Err(rterr!("Expected an exception to be thrown")),
                    Err(_) => {
                        globals.trace_unwind(trace_len);
                        Ok(Value::Nil)
                    }
                }
            }),
            NativeFunction::new("assert", ["cond"], None, |_globals, args, _| {
                let mut args = args.into_iter();
                let cond = args.next().unwrap();
                if !cond.truthy() {
                    Err(rterr!("Assertion failed"))
                } else {
                    Ok(Value::Nil)
                }
            }),
            NativeFunction::new("getattr", ["owner", "name"], None, |_globals, args, _| {
                let mut args = args.into_iter();
                let owner = args.next().unwrap();
                let name = args.next().unwrap().into_string()?;
                owner.getattr(&name)
            }),
            NativeFunction::new(
                "getattrs",
                ["owner"],
                "Returns a list of attributes on this value",
                |_globals, args, _| {
                    let mut args = args.into_iter();
                    let owner = args.next().unwrap();
                    Ok(Value::from(
                        owner
                            .getattrs()
                            .into_iter()
                            .map(Value::from)
                            .collect::<Vec<_>>(),
                    ))
                },
            ),
            NativeFunction::new("getmethod", ["cls", "name"], None, |_globals, args, _| {
                let mut args = args.into_iter();
                let cls = args.next().unwrap().into_class()?;
                let name = args.next().unwrap().into_string()?;
                cls.map()
                    .get(&name)
                    .cloned()
                    .ok_or_else(|| rterr!("Method {:?} not found for {:?}", name, cls))
            }),
            NativeFunction::new("getmethods", ["cls"], None, |_globals, args, _| {
                let mut args = args.into_iter();
                let cls = args.next().unwrap().into_class()?;
                let mut keys = cls.map().keys().collect::<Vec<_>>();
                keys.sort();
                Ok(Value::from(
                    keys.into_iter().map(Value::from).collect::<Vec<_>>(),
                ))
            }),
            NativeFunction::new(
                "throw",
                ["message"],
                concat!(
                    "Makeshift function for throwing runtime exceptions\n",
                    "API of this function is likely to change a lot in the future\n",
                ),
                |_globals, args, _| {
                    let mut args = args.into_iter();
                    let error = Error::try_from(args.next().unwrap())?;
                    Err(error)
                },
            ),
            NativeFunction::new(
                "pcall",
                ["f", "on_error"],
                concat!(
                    "Makeshift function for handling exceptions\n",
                    "Calls the function passed as the first argument.\n",
                    "If it finishes without any errors, this function will return that value.\n",
                    "If it throws, the second argument is called with the exception information ",
                    "and whatever is returned from it is returned.\n",
                    "API of this function is likely to change a lot in the future\n",
                ),
                |globals, args, _| {
                    let trace_len = globals.trace().len();
                    let mut args = args.into_iter();
                    let f = args.next().unwrap();
                    let on_error = args.next().unwrap();
                    match f.apply(globals, vec![], None) {
                        Ok(value) => Ok(value),
                        Err(error) => {
                            // we wait on unwinding the stack trace, so that the stack
                            // can be inspected from inside the error handler
                            // If we error from inside the handler, we let the
                            // trace accumulate.
                            let value = Value::from(error);
                            // We also put in an extra trace element
                            // so that we get an indication if an error occurred inside
                            // the error handler
                            globals.trace_push(Mark::new(
                                Source::new(
                                    "[pcall]".into(),
                                    None,
                                    "<while handling error...>".into(),
                                )
                                .into(),
                                0,
                                0,
                            ));
                            let result = on_error.apply(globals, vec![value], None)?;
                            globals.trace_unwind(trace_len);
                            Ok(result)
                        }
                    }
                },
            ),
            NativeFunction::new("hash", ["x"], None, |_globals, args, _| {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::Hash;
                use std::hash::Hasher;
                let mut args = args.into_iter();
                let x = args.next().unwrap();
                let key = Key::try_from(x)?;
                let mut s = DefaultHasher::new();
                key.hash(&mut s);
                Ok(Value::from(s.finish()))
            }),
            NativeFunction::new("type", ["x"], None, |globals, args, _| {
                let mut args = args.into_iter();
                let x = args.next().unwrap();
                Ok(Value::from(x.get_class(globals)))
            }),
            NativeFunction::new("int", ["x"], None, |_globals, args, _| {
                let mut args = args.into_iter();
                let x = args.next().unwrap();
                Ok(Value::from(x.convert_to_int()?))
            }),
            NativeFunction::new("float", ["x"], None, |_globals, args, _| {
                let mut args = args.into_iter();
                let x = args.next().unwrap();
                Ok(Value::from(x.convert_to_float()?))
            }),
            NativeFunction::new(
                "max",
                ArgSpec::builder().req("xs").var("varargs"),
                concat!(
                    "Return the largest item in an iterable or the largest of ",
                    "two or more arguments.",
                ),
                |globals, args, _| {
                    let (mut best, args) = if args.len() == 1 {
                        let args = args.into_iter().next().unwrap().unpack(globals)?;
                        if args.is_empty() {
                            return Err(rterr!("max of zero length sequence"));
                        }
                        let mut args = args.into_iter();
                        let curr = args.next().unwrap();
                        (curr, args)
                    } else {
                        let mut args = args.into_iter();
                        let curr = args.next().unwrap();
                        (curr, args)
                    };
                    for arg in args {
                        if best.lt(&arg)? {
                            best = arg;
                        }
                    }
                    Ok(best)
                },
            ),
            NativeFunction::new(
                "min",
                ArgSpec::builder().req("xs").var("varargs"),
                concat!(
                    "Return the smallest item in an iterable or the smallest of ",
                    "two or more arguments.",
                ),
                |globals, args, _| {
                    let (mut best, args) = if args.len() == 1 {
                        let args = args.into_iter().next().unwrap().unpack(globals)?;
                        if args.is_empty() {
                            return Err(rterr!("min of zero length sequence"));
                        }
                        let mut args = args.into_iter();
                        let curr = args.next().unwrap();
                        (curr, args)
                    } else {
                        let mut args = args.into_iter();
                        let curr = args.next().unwrap();
                        (curr, args)
                    };
                    for arg in args {
                        if arg.lt(&best)? {
                            best = arg;
                        }
                    }
                    Ok(best)
                },
            ),
            NativeFunction::new("__import", ["name"], None, |globals, args, _| {
                let name = args.into_iter().next().unwrap();
                let name = name.string()?;
                Ok(globals.load(name)?.into())
            }),
            NativeFunction::new("__disasm", ["func"], None, |_globals, args, _| {
                let func = args.into_iter().next().unwrap();
                let func = func.function()?;
                let string = func.code().disasm()?;
                Ok(Value::from(string))
            }),
            NativeFunction::new("__main", (), None, |globals, _args, _| {
                Ok(globals
                    .get_main()
                    .as_ref()
                    .map(Value::from)
                    .unwrap_or(Value::Nil))
            }),
        ];

        for func in funcs {
            map.insert(func.name().clone(), func.into());
        }

        map
    }
}
