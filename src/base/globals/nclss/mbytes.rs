use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::NativeFunction;
use crate::SymbolRegistryHandle;
use crate::Value;
use std::rc::Rc;
use std::cell::RefCell;

pub(super) fn mkcls(sr: &SymbolRegistryHandle, base: Rc<Class>) -> Rc<Class> {
    let methods = vec![
        NativeFunction::sdnew0(sr, "len", &["self"], None, |globals, args, _kwargs| {
            let bytes = Eval::expect_mutable_bytes(globals, &args[0])?;
            Ok((bytes.borrow().len() as i64).into())
        }),
        NativeFunction::simple0(sr, "__getitem", &["self", "i"], |globals, args, _kwargs| {
            let bytes = Eval::expect_mutable_bytes(globals, &args[0])?;
            let i = Eval::expect_index(globals, &args[1], bytes.borrow().len())?;
            Ok((bytes.borrow()[i] as i64).into())
        }),
        NativeFunction::simple0(sr, "extend", &["self", "other"], |globals, args, _kwargs| {
            let bytes = Eval::expect_mutable_bytes(globals, &args[0])?;
            let other = Eval::expect_bytes_from_pattern(globals, &args[1])?;
            bytes.borrow_mut().extend(other);
            Ok(Value::Nil)
        }),
        NativeFunction::simple0(sr, "move", &["self"], |globals, args, _kwargs| {
            let bytes = Eval::expect_mutable_bytes(globals, &args[0])?;
            let bytes = bytes.replace(vec![]);
            Ok(bytes.into())
        }),
        NativeFunction::sdnew0(
            sr,
            "__slice",
            &["self", "start", "end"],
            Some("Creates a new bytes object consisting of a subrange of this object"),
            |globals, args, _kwargs| {
                let bytes = Eval::expect_mutable_bytes(globals, &args[0])?;
                let (start, end) =
                    Eval::expect_range_indices(globals, &args[1], &args[2], bytes.borrow().len())?;
                Ok(bytes.borrow()[start..end].to_vec().into())
            },
        ),
    ]
    .into_iter()
    .map(|f| (sr.intern_rcstr(f.name()), Value::from(f)))
    .collect();

    let static_methods = vec![
        NativeFunction::simple0(sr, "__call", &["pattern"], |globals, args, _kwargs| {
            let bytes = Eval::expect_bytes_from_pattern(globals, &args[0])?;
            Ok(Value::MutableBytes(Rc::new(RefCell::new(bytes))))
        }),
        NativeFunction::simple0(
            sr,
            "from_iterable",
            &["iterable"],
            |globals, args, _kwargs| {
                let bytes = Eval::expect_bytes_from_pattern(globals, &args[0])?;
                Ok(Value::MutableBytes(Rc::new(RefCell::new(bytes))))
            },
        ),
    ]
    .into_iter()
    .map(|f| (sr.intern_rcstr(f.name()), Value::from(f)))
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
