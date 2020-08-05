use crate::ParameterInfo;
use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::NativeFunction;
use crate::Symbol;
use crate::Value;

use std::collections::HashMap;
use std::rc::Rc;

pub(super) fn mkcls(base: Rc<Class>) -> Rc<Class> {
    let methods = vec![
        NativeFunction::new("keys", &["self"], None, |globals, args, _kwargs| {
            let m = Eval::expect_module(globals, &args[0])?;
            let keys: Vec<_> = m.keys().map(|s| Value::from(*s)).collect();
            Ok(keys.into())
        }),
        NativeFunction::new(
            "get",
            ParameterInfo::builder().required("self").required("key").optional("default", Value::Uninitialized),
            None,
            |globals, args, _kwargs| {
                let key = Eval::expect_symbollike(globals, &args[1])?;
                if let Value::Uninitialized = &args[2] {
                    Eval::get_static_attr_or_err(globals, &args[0], key)
                } else {
                    match Eval::get_static_attr(globals, &args[0], key) {
                        Some(value) => Ok(value),
                        None => Ok(args[2].clone()),
                    }
                }
            },
        ),
        NativeFunction::new(
            "doc",
            ParameterInfo::builder().required("self").optional("key", Value::Uninitialized),
            None,
            |globals, args, _kwargs| {
                let m = Eval::expect_module(globals, &args[0])?;
                if let Value::Uninitialized = &args[1] {
                    let doc = m.doc();
                    match doc {
                        Some(doc) => Ok(doc.clone().into()),
                        None => Ok(Value::Nil),
                    }
                } else {
                    let key = Eval::expect_symbol(globals, &args[1])?;
                    match m.member_doc(globals, key)? {
                        Some(doc) => Ok(doc.clone().into()),
                        None => Ok(Value::Nil),
                    }
                }
            },
        ),
    ]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
    .collect();

    let static_methods = HashMap::new();

    Class::new0(
        ClassKind::NativeClass,
        "Module".into(),
        vec![base],
        None,
        methods,
        static_methods,
    )
    .into()
}
