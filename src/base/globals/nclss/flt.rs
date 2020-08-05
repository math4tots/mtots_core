use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::NativeFunction;
use crate::Symbol;
use crate::Value;
use std::collections::HashMap;
use std::rc::Rc;

pub(super) fn mkcls(base: Rc<Class>) -> Rc<Class> {
    let static_methods = vec![NativeFunction::new(
        "__call",
        &["x"],
        "Converts a value to a Number",
        |globals, args, _kwargs| match &args[0] {
            Value::Number(f) => Ok(Value::Number(*f)),
            Value::String(s) => match s.str().parse() {
                Ok(i) => Ok(Value::Number(i)),
                Err(error) => {
                    globals.set_exc_str(&format!("Float parse failed: {:?}, {:?}", s, error))
                }
            },
            _ => Ok(Eval::expect_int(globals, &args[0])?.into()),
        },
    )]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
    .collect();

    Class::new0(
        ClassKind::NativeClass,
        "Number".into(),
        vec![base],
        None,
        HashMap::new(),
        static_methods,
    )
    .into()
}
