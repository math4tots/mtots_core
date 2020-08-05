use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::NativeFunction;
use crate::Symbol;
use crate::Value;
use std::cell::RefCell;
use std::rc::Rc;

pub(super) fn mkcls(base: Rc<Class>) -> Rc<Class> {
    let methods = vec![
        NativeFunction::new("len", &["self"], None, |globals, args, _kwargs| {
            let bytes = Eval::expect_mutable_bytes(globals, &args[0])?;
            Ok((bytes.borrow().len() as i64).into())
        }),
        NativeFunction::new(
            "__getitem",
            &["self", "i"],
            None,
            |globals, args, _kwargs| {
                let bytes = Eval::expect_mutable_bytes(globals, &args[0])?;
                let i = Eval::expect_index(globals, &args[1], bytes.borrow().len())?;
                Ok((bytes.borrow()[i] as i64).into())
            },
        ),
        NativeFunction::new(
            "extend",
            &["self", "other"],
            None,
            |globals, args, _kwargs| {
                let bytes = Eval::expect_mutable_bytes(globals, &args[0])?;
                let other = Eval::expect_bytes_from_pattern(globals, &args[1])?;
                bytes.borrow_mut().extend(other);
                Ok(Value::Nil)
            },
        ),
        NativeFunction::new("move", &["self"], None, |globals, args, _kwargs| {
            let bytes = Eval::expect_mutable_bytes(globals, &args[0])?;
            let bytes = bytes.replace(vec![]);
            Ok(bytes.into())
        }),
        NativeFunction::new(
            "__slice",
            &["self", "start", "end"],
            "Creates a new bytes object consisting of a subrange of this object",
            |globals, args, _kwargs| {
                let bytes = Eval::expect_mutable_bytes(globals, &args[0])?;
                let (start, end) =
                    Eval::expect_range_indices(globals, &args[1], &args[2], bytes.borrow().len())?;
                Ok(bytes.borrow()[start..end].to_vec().into())
            },
        ),
    ]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
    .collect();

    let static_methods = vec![
        NativeFunction::new("__call", &["pattern"], None, |globals, args, _kwargs| {
            let bytes = Eval::expect_bytes_from_pattern(globals, &args[0])?;
            Ok(Value::MutableBytes(Rc::new(RefCell::new(bytes))))
        }),
        NativeFunction::new(
            "from_iterable",
            &["iterable"],
            None,
            |globals, args, _kwargs| {
                let bytes = Eval::expect_bytes_from_pattern(globals, &args[0])?;
                Ok(Value::MutableBytes(Rc::new(RefCell::new(bytes))))
            },
        ),
    ]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
    .collect();

    Class::new0(
        ClassKind::NativeClass,
        "MutableBytes".into(),
        vec![base],
        None,
        methods,
        static_methods,
    )
    .into()
}
