use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::NativeFunction;
use crate::Value;
use crate::Symbol;
use std::collections::HashMap;
use std::rc::Rc;

pub(super) fn mkcls(base: Rc<Class>) -> Rc<Class> {
    let methods = vec![
        NativeFunction::snew(
            "is_trait",
            (&["self"], &[], None, None),
            |globals, args, _kwargs| {
                let cls = Eval::expect_class(globals, &args[0])?;
                Ok(cls.is_trait().into())
            },
        ),
        NativeFunction::snew(
            "get_method",
            (
                &["self", "name"],
                &[("default", Value::Uninitialized)],
                None,
                None,
            ),
            |globals, args, _kwargs| {
                let cls = Eval::expect_class(globals, &args[0])?;
                let method_name = Eval::expect_symbollike(globals, &args[1])?;
                match cls.get_from_instance_map(&method_name) {
                    Some(method) => Ok(method.clone()),
                    None => {
                        if let Value::Uninitialized = &args[2] {
                            globals.set_exc_str(&format!(
                                "Method {} not found for class {}",
                                method_name,
                                cls.full_name(),
                            ))
                        } else {
                            Ok(args[2].clone())
                        }
                    }
                }
            },
        ),
        NativeFunction::sdnew0(
            "keys",
            &["self"],
            Some("Returns method names as a List of Symbols"),
            |globals, args, _kwargs| {
                let cls = Eval::expect_class(globals, &args[0])?;
                let mut names = Vec::new();
                for key in cls.instance_keys() {
                    names.push(Value::Symbol(key));
                }
                Ok(names.into())
            },
        ),
        NativeFunction::sdnew0(
            "static_keys",
            &["self"],
            Some("Returns static method names as a List of Symbols"),
            |globals, args, _kwargs| {
                let cls = Eval::expect_class(globals, &args[0])?;
                let mut names = Vec::new();
                for key in cls.static_keys() {
                    names.push(Value::Symbol(key));
                }
                Ok(names.into())
            },
        ),
        NativeFunction::snew(
            "doc",
            (&["self"], &[], None, None),
            |globals, args, _kwargs| {
                let cls = Eval::expect_class(globals, &args[0])?;
                match cls.doc() {
                    Some(doc) => Ok(doc.clone().into()),
                    None => Ok(Value::Nil),
                }
            },
        ),
    ]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
    .collect();

    Class::new0(
        ClassKind::NativeClass,
        "Class".into(),
        vec![base],
        Some(concat!(
            "Instances of Class represent classes and traits in a running\n",
            "interpreter application\n",
        )),
        methods,
        HashMap::new(),
    )
    .into()
}
