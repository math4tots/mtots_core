use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::NativeFunction;
use crate::SymbolRegistryHandle;
use crate::Value;
use std::collections::HashMap;
use std::rc::Rc;

pub(super) fn mkcls(sr: &SymbolRegistryHandle, base: Rc<Class>) -> Rc<Class> {
    let methods = vec![NativeFunction::snew(
        sr,
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
    )]
    .into_iter()
    .map(|f| (sr.intern_rcstr(f.name()), Value::from(f)))
    .collect();

    Class::new0(
        ClassKind::NativeClass,
        "Class".into(),
        vec![base],
        methods,
        HashMap::new(),
    )
    .into()
}
