use crate::divmod;
use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::NativeFunction;
use crate::Symbol;
use crate::Value;
use std::rc::Rc;

pub(super) fn mkcls(base: Rc<Class>) -> Rc<Class> {
    let methods = vec![
        NativeFunction::simple0("rem", &["self", "other"], |globals, args, _kwargs| {
            let a = Eval::expect_int(globals, &args[0])?;
            let b = Eval::expect_int(globals, &args[1])?;
            Ok(Value::Int(a % b))
        }),
        NativeFunction::simple0("tdiv", &["self", "other"], |globals, args, _kwargs| {
            let a = Eval::expect_int(globals, &args[0])?;
            let b = Eval::expect_int(globals, &args[1])?;
            Ok(Value::Int(a / b))
        }),
        NativeFunction::simple0("divrem", &["self", "other"], |globals, args, _kwargs| {
            let a = Eval::expect_int(globals, &args[0])?;
            let b = Eval::expect_int(globals, &args[1])?;
            Ok(vec![Value::Int(a / b), Value::Int(a % b)].into())
        }),
        NativeFunction::simple0("mod", &["self", "other"], |globals, args, _kwargs| {
            let a = Eval::expect_int(globals, &args[0])?;
            let b = Eval::expect_int(globals, &args[1])?;
            Ok(Value::Int(divmod(a, b).1))
        }),
        NativeFunction::simple0("fdiv", &["self", "other"], |globals, args, _kwargs| {
            let a = Eval::expect_int(globals, &args[0])?;
            let b = Eval::expect_int(globals, &args[1])?;
            Ok(Value::Int(divmod(a, b).0))
        }),
        NativeFunction::simple0("divmod", &["self", "other"], |globals, args, _kwargs| {
            let a = Eval::expect_int(globals, &args[0])?;
            let b = Eval::expect_int(globals, &args[1])?;
            let (d, m) = divmod(a, b);
            Ok(vec![Value::Int(d), Value::Int(m)].into())
        }),
    ]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
    .collect();

    let static_methods = vec![NativeFunction::sdnew0(
        "__call",
        &["x"],
        Some("Converts a value to an Int"),
        |globals, args, _kwargs| match &args[0] {
            Value::Int(i) => Ok(Value::Int(*i)),
            Value::Float(f) => Ok(Value::Int(*f as i64)),
            Value::String(s) => match s.str().parse() {
                Ok(i) => Ok(Value::Int(i)),
                Err(error) => {
                    globals.set_exc_str(&format!("Int parse failed: {:?}, {:?}", s, error))
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
        "Int".into(),
        vec![base],
        None,
        methods,
        static_methods,
    )
    .into()
}
