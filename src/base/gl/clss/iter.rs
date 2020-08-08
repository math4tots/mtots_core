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
                    "enumerate",
                    ArgSpec::builder().req("self").def("start", 0),
                    "Converts this iterator into a list",
                    |_globals, args, _| {
                        let mut args = args.into_iter();
                        let owner = args.next().unwrap();
                        let mut i = args.next().unwrap().number()?;
                        Ok(NativeGenerator::new(
                            "Iterator.enumerate",
                            move |globals, arg| {
                                match owner.resume(globals, arg) {
                                    ResumeResult::Yield(value) => {
                                        let n = i;
                                        i += 1.0;
                                        ResumeResult::Yield(vec![
                                            Value::from(n),
                                            value,
                                        ].into())
                                    }
                                    r => r,
                                }
                            },
                        ).into())
                    },
                ),
            ]),
            vec![iterable],
        ),
        HashMap::new(),
    )
}
