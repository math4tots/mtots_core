use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::NativeFunction;
use crate::SymbolRegistryHandle;
use crate::Value;
use std::collections::HashMap;
use std::rc::Rc;

pub(super) fn mkcls(sr: &SymbolRegistryHandle, base: Rc<Class>) -> Rc<Class> {
    let static_methods = vec![NativeFunction::sdnew0(
        sr,
        "__call",
        &["x"],
        Some("Converts a value to an Int"),
        |globals, args, _kwargs| match &args[0] {
            Value::Int(i) => Ok(Value::Float(*i as f64)),
            Value::Float(f) => Ok(Value::Float(*f)),
            Value::String(s) => match s.str().parse() {
                Ok(i) => Ok(Value::Float(i)),
                Err(error) => {
                    globals.set_exc_str(&format!("Float parse failed: {:?}, {:?}", s, error))
                }
            },
            _ => Ok(Eval::expect_int(globals, &args[0])?.into()),
        },
    )]
    .into_iter()
    .map(|f| (sr.intern_rcstr(f.name()), Value::from(f)))
    .collect();

    Class::new0(
        ClassKind::NativeClass,
        "Float".into(),
        vec![base],
        None,
        HashMap::new(),
        static_methods,
    )
    .into()
}
