use super::*;

#[allow(non_snake_case)]
pub struct ClassManager {
    pub Nil: Rc<Class>,
    pub Bool: Rc<Class>,
    pub Number: Rc<Class>,
    pub String: Rc<Class>,
    pub List: Rc<Class>,
    pub Set: Rc<Class>,
    pub Map: Rc<Class>,
    pub Function: Rc<Class>,
    pub NativeFunction: Rc<Class>,
    pub Generator: Rc<Class>,
    pub NativeGenerator: Rc<Class>,
    pub Class: Rc<Class>,
    pub Module: Rc<Class>,
}

impl ClassManager {
    #[allow(non_snake_case)]
    pub(super) fn new() -> Self {
        let Nil = Class::new(
            "Nil".into(),
            Class::map_from_funcs(vec![
                NativeFunction::new("map", ["self", "x", "f"], "", |globals, args, _| {
                    let mut args = args.into_iter();
                    args.next().unwrap(); // nil
                    let x = args.next().unwrap();
                    let f = args.next().unwrap();
                    if x.is_nil() {
                        Ok(Value::Nil)
                    } else {
                        f.apply(globals, vec![x], None)
                    }
                }),
                NativeFunction::new("vmap", ["self", "x", "new"], "", |_globals, args, _| {
                    let mut args = args.into_iter();
                    args.next().unwrap(); // nil
                    let x = args.next().unwrap();
                    let new = args.next().unwrap();
                    if x.is_nil() {
                        Ok(Value::Nil)
                    } else {
                        Ok(new)
                    }
                }),
                NativeFunction::new("get", ["self", "x", "default"], "", |_globals, args, _| {
                    let mut args = args.into_iter();
                    args.next().unwrap(); // nil
                    let x = args.next().unwrap();
                    let default = args.next().unwrap();
                    if x.is_nil() {
                        Ok(default)
                    } else {
                        Ok(x)
                    }
                }),
                NativeFunction::new("fget", ["self", "x", "f"], "", |globals, args, _| {
                    let mut args = args.into_iter();
                    args.next().unwrap(); // nil
                    let x = args.next().unwrap();
                    let f = args.next().unwrap();
                    if x.is_nil() {
                        f.apply(globals, vec![], None)
                    } else {
                        Ok(x)
                    }
                }),
            ]),
            HashMap::new(),
        );
        let Bool = Class::new("Bool".into(), HashMap::new(), HashMap::new());
        let Number = Class::new("Number".into(), HashMap::new(), HashMap::new());
        let String = Class::new(
            "String".into(),
            Class::map_from_funcs(vec![
                NativeFunction::new(
                    "len",
                    ["self"],
                    concat!(
                        "Returns the number of unicode codepoints\n\n",
                        "Note this is similar to the length of a string in Python ",
                        "and different from length of a string in other languages ",
                        "like Rust where the length of a string is the number ",
                        "of bytes"
                    ),
                    |_globals, args, _| {
                        let mut args = args.into_iter();
                        let owner = args.next().unwrap().into_string()?;
                        Ok(Value::from(owner.charlen()))
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
            ]),
            HashMap::new(),
        );
        let List = Class::new(
            "List".into(),
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
            HashMap::new(),
        );
        let Set = Class::new(
            "Set".into(),
            HashMap::new(),
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
        );
        let Map = Class::new("Map".into(), HashMap::new(), HashMap::new());
        let Function = Class::new(
            "Function".into(),
            Class::map_from_funcs(vec![
                NativeFunction::new("params", ["self"], None, |_globals, args, _| {
                    let f = args.into_iter().next().unwrap().into_function()?;
                    Ok(f.argspec().to_value())
                }),
                NativeFunction::new("doc", ["self"], None, |_globals, args, _| {
                    let f = args.into_iter().next().unwrap().into_function()?;
                    Ok(f.doc().as_ref().map(Value::from).unwrap_or(Value::Nil))
                }),
            ]),
            HashMap::new(),
        );
        let NativeFunction = Class::new(
            "NativeFunction".into(),
            Class::map_from_funcs(vec![
                NativeFunction::new("params", ["self"], None, |_globals, args, _| {
                    let f = args.into_iter().next().unwrap().into_native_function()?;
                    Ok(f.argspec().to_value())
                }),
                NativeFunction::new("doc", ["self"], None, |_globals, args, _| {
                    let f = args.into_iter().next().unwrap().into_native_function()?;
                    Ok(f.doc().as_ref().map(Value::from).unwrap_or(Value::Nil))
                }),
            ]),
            HashMap::new(),
        );
        let Generator = Class::new("Generator".into(), HashMap::new(), HashMap::new());
        let NativeGenerator = Class::new("NativeGenerator".into(), HashMap::new(), HashMap::new());
        let Class = Class::new("Class".into(), HashMap::new(), HashMap::new());
        let Module = Class::new("Module".into(), HashMap::new(), HashMap::new());
        Self {
            Nil,
            Bool,
            Number,
            String,
            List,
            Set,
            Map,
            Function,
            NativeFunction,
            Generator,
            NativeGenerator,
            Class,
            Module,
        }
    }

    pub fn get_class<'a>(&'a self, value: &'a Value) -> &'a Rc<Class> {
        match value {
            Value::Invalid => panic!("get_class(Invalid)"),
            Value::Nil => &self.Nil,
            Value::Bool(..) => &self.Bool,
            Value::Number(..) => &self.Number,
            Value::String(..) => &self.String,
            Value::List(..) => &self.List,
            Value::Set(..) => &self.Set,
            Value::Map(..) => &self.Map,
            Value::Table(table) => table.cls(),
            Value::Function(..) => &self.Function,
            Value::NativeFunction(..) => &self.NativeFunction,
            Value::Generator(..) => &self.Generator,
            Value::NativeGenerator(..) => &self.NativeGenerator,
            Value::Class(..) => &self.Class,
            Value::Module(..) => &self.Module,
            Value::Handle(handle) => handle.cls(),
        }
    }

    pub fn list(&self) -> Vec<&Rc<Class>> {
        vec![
            &self.Nil,
            &self.Bool,
            &self.Number,
            &self.String,
            &self.List,
            &self.Set,
            &self.Map,
            &self.Function,
            &self.NativeFunction,
            &self.Generator,
            &self.NativeGenerator,
            &self.Class,
            &self.Module,
        ]
    }
}
