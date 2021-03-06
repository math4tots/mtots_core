use super::*;

pub(super) fn new() -> Rc<Class> {
    Class::new(
        "String".into(),
        Class::map_from_funcs(vec![
            NativeFunction::new(
                "len",
                ["self"],
                concat!(
                    "Returns the number of bytes in the UTF-8 representation of ",
                    "this string\n\n",
                ),
                |_globals, args, _| {
                    let mut args = args.into_iter();
                    let owner = args.next().unwrap().into_string()?;
                    Ok(Value::from(owner.len()))
                },
            ),
            NativeFunction::new(
                "__add",
                ["self", "other"],
                "Concatenates the two strings",
                |_globals, args, _| {
                    let mut args = args.into_iter();
                    let mut owner = args.next().unwrap().into_string()?.unwrap_or_clone();
                    let other = args.next().unwrap().into_string()?;
                    owner.push_str(other.str());
                    Ok(Value::from(owner))
                },
            ),
            NativeFunction::new(
                "join",
                ["self", "parts"],
                "Returns a new string with the old pattern replaced with the new",
                |globals, args, _| {
                    let mut args = args.into_iter();
                    let owner = args.next().unwrap().into_string()?;
                    let mut parts = args.next().unwrap().unpack(globals)?.into_iter();
                    let mut ret = String::new();
                    if let Some(part) = parts.next() {
                        ret.push_str(part.into_string()?.str());
                        while let Some(part) = parts.next() {
                            ret.push_str(owner.str());
                            ret.push_str(part.into_string()?.str());
                        }
                    }
                    Ok(Value::from(ret))
                },
            ),
            NativeFunction::new(
                "replace",
                ["self", "old", "new"],
                "Returns a new string with the old pattern replaced with the new",
                |_globals, args, _| {
                    let mut args = args.into_iter();
                    let owner = args.next().unwrap();
                    let owner = owner.string()?;
                    let old = args.next().unwrap();
                    let old = old.string()?;
                    let new = args.next().unwrap();
                    let new = new.string()?;
                    Ok(owner.replace(old.str(), new.str()).into())
                },
            ),
            NativeFunction::new(
                "find",
                ArgSpec::builder()
                    .req("self")
                    .req("pattern")
                    .def("start", ())
                    .def("end", ()),
                "",
                |_globals, args, _| {
                    let mut args = args.into_iter();
                    let owner = args.next().unwrap().into_string()?;
                    let len = owner.len();
                    let pattern = args.next().unwrap().into_string()?;
                    let start = args.next().unwrap().to_start_index(len)?;
                    let end = args.next().unwrap().to_end_index(len)?;
                    Ok(owner[start..end]
                        .find(pattern.str())
                        .map(|i| start + i)
                        .map(Value::from)
                        .unwrap_or(Value::Nil))
                },
            ),
            NativeFunction::new(
                "rfind",
                ArgSpec::builder()
                    .req("self")
                    .req("pattern")
                    .def("start", ())
                    .def("end", ()),
                "",
                |_globals, args, _| {
                    let mut args = args.into_iter();
                    let owner = args.next().unwrap().into_string()?;
                    let len = owner.len();
                    let pattern = args.next().unwrap().into_string()?;
                    let start = args.next().unwrap().to_start_index(len)?;
                    let end = args.next().unwrap().to_end_index(len)?;
                    Ok(owner[start..end]
                        .rfind(pattern.str())
                        .map(|i| start + i)
                        .map(Value::from)
                        .unwrap_or(Value::Nil))
                },
            ),
            NativeFunction::new("__rem", ["self", "args"], "", |globals, args, _| {
                let mut args = args.into_iter();
                let owner = args.next().unwrap().into_string()?;
                let args = args.next().unwrap().unpack(globals)?;
                Ok(Value::from(Value::format_string(owner.str(), args)?))
            }),
            NativeFunction::new(
                "__mul",
                ["self", "n"],
                "Repeats a string n times",
                |_globals, args, _| {
                    let mut args = args.into_iter();
                    let owner = args.next().unwrap().into_string()?;
                    let n = usize::try_from(args.next().unwrap())?;
                    let mut ret = String::new();
                    for _ in 0..n {
                        ret.push_str(&owner);
                    }
                    Ok(ret.into())
                },
            ),
            NativeFunction::new("words", ["self"], "", |_globals, args, _| {
                let mut args = args.into_iter();
                let owner = args.next().unwrap().into_string()?;
                let len = owner.len();
                let mut i = 0;
                let mut fin = false;
                Ok(NativeGenerator::new("String.words", move |_globals, _| {
                    if fin {
                        ResumeResult::Return(Value::Nil)
                    } else {
                        let j = owner[i..]
                            .find(char::is_whitespace)
                            .map(|j| i + j)
                            .unwrap_or(len);
                        let next = Value::from(&owner[i..j]);
                        i = owner[j..]
                            .find(|c: char| !c.is_whitespace())
                            .map(|i| i + j)
                            .unwrap_or(len);
                        if j == len {
                            fin = true;
                        }
                        ResumeResult::Yield(next)
                    }
                })
                .into())
            }),
            NativeFunction::new("lines", ["self"], "", |_globals, args, _| {
                let mut args = args.into_iter();
                let owner = args.next().unwrap().into_string()?;
                let len = owner.len();
                let mut i = 0;
                let mut fin = false;
                Ok(NativeGenerator::new("String.lines", move |_globals, _| {
                    if fin {
                        ResumeResult::Return(Value::Nil)
                    } else {
                        let j = owner[i..]
                            .find(|c: char| c == '\n')
                            .map(|j| i + j)
                            .unwrap_or(len);
                        let next = Value::from(&owner[i..j]);
                        i = j + 1;
                        if j == len {
                            fin = true;
                        }
                        ResumeResult::Yield(next)
                    }
                })
                .into())
            }),
            NativeFunction::new("split", ["self", "sep"], "", |_globals, args, _| {
                let mut args = args.into_iter();
                let owner = args.next().unwrap().into_string()?;
                let sep = args.next().unwrap().into_string()?;
                let len = owner.len();
                let mut i = 0;
                let mut fin = false;
                Ok(NativeGenerator::new("String.split", move |_globals, _| {
                    if fin {
                        ResumeResult::Return(Value::Nil)
                    } else {
                        let j = owner[i..].find(sep.str()).map(|j| i + j).unwrap_or(len);
                        let next = Value::from(&owner[i..j]);
                        i = j + sep.len();
                        if j == len {
                            fin = true;
                        }
                        ResumeResult::Yield(next)
                    }
                })
                .into())
            }),
            NativeFunction::new(
                "slice",
                ["self", "start", "end"],
                "",
                |_globals, args, _| {
                    let mut args = args.into_iter();
                    let owner = args.next().unwrap().into_string()?;
                    let len = owner.len();
                    let start = args.next().unwrap().to_start_index(len)?;
                    let end = args.next().unwrap().to_end_index(len)?;
                    Ok(Value::from(&owner[start..end]))
                },
            ),
            NativeFunction::new(
                "__slice",
                ["self", "start", "end"],
                "",
                |_globals, args, _| {
                    let mut args = args.into_iter();
                    let owner = args.next().unwrap().into_string()?;
                    let len = owner.len();
                    let start = args.next().unwrap().to_start_index(len)?;
                    let end = args.next().unwrap().to_end_index(len)?;
                    Ok(Value::from(&owner[start..end]))
                },
            ),
            NativeFunction::new(
                "starts_with",
                ["self", "prefix"],
                "",
                |_globals, args, _| {
                    let mut args = args.into_iter();
                    let owner = args.next().unwrap().into_string()?;
                    let prefix = args.next().unwrap().into_string()?;
                    Ok(owner.starts_with(prefix.str()).into())
                },
            ),
            NativeFunction::new("ends_with", ["self", "suffix"], "", |_globals, args, _| {
                let mut args = args.into_iter();
                let owner = args.next().unwrap().into_string()?;
                let suffix = args.next().unwrap().into_string()?;
                Ok(owner.ends_with(suffix.str()).into())
            }),
            NativeFunction::new(
                "trim",
                ["self"],
                "Returns self with surrounding whitespace removed",
                |_globals, args, _| {
                    let mut args = args.into_iter();
                    let owner = args.next().unwrap().into_string()?;
                    Ok(owner.trim().into())
                },
            ),
            NativeFunction::new(
                "rstrip",
                ["self", "suffix"],
                "Returns self with suffix removed, if it ends with the given suffix",
                |_globals, args, _| {
                    let mut args = args.into_iter();
                    let owner = args.next().unwrap().into_string()?;
                    let suffix = args.next().unwrap().into_string()?;
                    if owner.ends_with(suffix.str()) {
                        let stripped = &owner[..owner.len() - suffix.len()];
                        Ok(stripped.into())
                    } else {
                        Ok(owner.into())
                    }
                },
            ),
            NativeFunction::new(
                "lstrip",
                ["self", "prefix"],
                "Returns self with prefix removed, if it starts with the given prefix",
                |_globals, args, _| {
                    let mut args = args.into_iter();
                    let owner = args.next().unwrap().into_string()?;
                    let prefix = args.next().unwrap().into_string()?;
                    if owner.starts_with(prefix.str()) {
                        let stripped = &owner[prefix.len()..];
                        Ok(stripped.into())
                    } else {
                        Ok(owner.into())
                    }
                },
            ),
            NativeFunction::new(
                "chars",
                ArgSpec::builder().req("self").def("start", 0),
                "",
                |_globals, args, _| {
                    let mut args = args.into_iter();
                    let owner = args.next().unwrap().into_string()?;
                    let mut i = usize::try_from(args.next().unwrap())?;
                    let len = owner.len();
                    Ok(NativeGenerator::new("String.chars", move |_globals, _| {
                        while i < len && !owner.is_char_boundary(i) {
                            i += 1;
                        }
                        if i < len {
                            let ch = owner[i..].chars().next().unwrap();
                            i += 1;
                            ResumeResult::Yield(ch.into())
                        } else {
                            ResumeResult::Return(Value::Nil)
                        }
                    })
                    .into())
                },
            ),
        ]),
        HashMap::new(),
    )
}
