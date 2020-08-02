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
        NativeFunction::simple0("doc", &["self"], |globals, args, _kwargs| {
            let func = Eval::expect_func(globals, &args[0])?;
            match func.doc() {
                Some(doc) => Ok(doc.clone().into()),
                None => Ok(Value::Nil),
            }
        }),
        NativeFunction::simple0("params", &["self"], |globals, args, _kwargs| {
            let func = Eval::expect_func(globals, &args[0])?;
            let pi = func.parameter_info();
            Eval::parameter_info_to_value(globals, pi)
        }),
        NativeFunction::simple0("__asm_str", &["self"], |globals, args, _kwargs| {
            let func = Eval::expect_func(globals, &args[0])?;
            Ok(func.disasm_str().into())
        }),
    ]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
    .collect();

    Class::new0(
        ClassKind::NativeClass,
        "Function".into(),
        vec![base],
        None,
        methods,
        HashMap::new(),
    )
    .into()
}
