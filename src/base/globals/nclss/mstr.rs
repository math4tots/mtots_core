use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::NativeFunction;
use crate::RcStr;
use crate::Value;
use crate::Symbol;
use std::cell::RefCell;
use std::rc::Rc;

pub(super) fn mkcls(base: Rc<Class>) -> Rc<Class> {
    let methods = vec![
        NativeFunction::simple0("len", &["self"], |globals, args, _kwargs| {
            let s = Eval::expect_mutable_string(globals, &args[0])?;
            Ok(Value::Int(s.borrow().len() as i64))
        }),
        NativeFunction::simple0(
            "extend",
            &["self", "other"],
            |globals, args, _kwargs| {
                let s = Eval::expect_mutable_string(globals, &args[0])?;
                Eval::extend_str(globals, &mut s.borrow_mut(), &args[1])?;
                Ok(Value::Nil)
            },
        ),
        NativeFunction::simple0("move", &["self"], |globals, args, _kwargs| {
            let mstr = Eval::expect_mutable_string(globals, &args[0])?;
            let contents = mstr.replace(String::new());
            Ok(contents.into())
        }),
    ]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
    .collect();

    let static_methods = vec![NativeFunction::simple0(
        "__call",
        &["x"],
        |globals, args, _kwargs| {
            Ok(Value::MutableString(
                RefCell::new(RcStr::unwrap_or_clone(Eval::str(globals, &args[0])?)).into(),
            ))
        },
    )]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
    .collect();

    Class::new0(
        ClassKind::NativeClass,
        "MutableString".into(),
        vec![base],
        None,
        methods,
        static_methods,
    )
    .into()
}
