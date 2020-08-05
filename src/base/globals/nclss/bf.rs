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
        NativeFunction::new("doc", &["self"], None, |globals, args, _kwargs| {
            let func = Eval::expect_native_func(globals, &args[0])?;
            match func.doc() {
                Some(doc) => Ok(doc.clone().into()),
                None => Ok(Value::Nil),
            }
        }),
        NativeFunction::new("params", &["self"], None, |globals, args, _kwargs| {
            let func = Eval::expect_native_func(globals, &args[0])?;
            let pi = func.parameter_info();
            Eval::parameter_info_to_value(globals, pi)
        }),
    ]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
    .collect();

    Class::new0(
        ClassKind::NativeClass,
        "NativeFunction".into(),
        vec![base],
        None,
        methods,
        HashMap::new(),
    )
    .into()
}
