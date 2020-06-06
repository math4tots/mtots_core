use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::NativeFunction;
use crate::SymbolRegistryHandle;
use crate::Value;
use std::cell::RefCell;
use std::rc::Rc;

pub(super) fn mkcls(sr: &SymbolRegistryHandle, base: Rc<Class>) -> Rc<Class> {
    let methods = vec![
        NativeFunction::simple0(sr, "move", &["self"], |globals, args, _kwargs| {
            // Like MutableString:move -- empties out the contents of this List,
            // while returning an immutable List with the same contents
            let list = Eval::expect_mutable_list(globals, &args[0])?;
            let contents = list.replace(vec![]);
            Ok(contents.into())
        }),
        NativeFunction::simple0(sr, "len", &["self"], |globals, args, _kwargs| {
            let list = Eval::expect_mutable_list(globals, &args[0])?;
            Ok(Value::Int(list.borrow().len() as i64))
        }),
        NativeFunction::simple0(sr, "__getitem", &["self", "i"], |globals, args, _kwargs| {
            let list = Eval::expect_mutable_list(globals, &args[0])?;
            let i = Eval::expect_index(globals, &args[1], list.borrow().len())?;
            Ok((list.borrow())[i].clone())
        }),
        NativeFunction::simple0(
            sr,
            "__setitem",
            &["self", "i", "val"],
            |globals, args, _kwargs| {
                let list = Eval::expect_mutable_list(globals, &args[0])?;
                let i = Eval::expect_index(globals, &args[1], list.borrow().len())?;
                let val = &args[2];
                list.borrow_mut()[i] = val.clone();
                Ok(Value::Nil)
            },
        ),
        NativeFunction::simple0(sr, "map", &["self", "f"], |globals, args, _kwargs| {
            let list = Eval::expect_mutable_list(globals, &args[0])?;
            let f = &args[1];
            let mut ret = Vec::new();
            for x in list.borrow().iter() {
                let y = Eval::call(globals, f, vec![x.clone()])?;
                ret.push(y);
            }
            Ok(Value::MutableList(RefCell::new(ret).into()))
        }),
        NativeFunction::simple0(sr, "pop", &["self"], |globals, args, _kwargs| {
            let list = Eval::expect_mutable_list(globals, &args[0])?;
            match list.borrow_mut().pop() {
                Some(value) => Ok(value),
                None => return globals.set_empty_pop_error()?,
            }
        }),
        NativeFunction::simple0(sr, "push", &["self", "x"], |globals, args, _kwargs| {
            let list = Eval::expect_mutable_list(globals, &args[0])?;
            let item = args[1].clone();
            list.borrow_mut().push(item);
            Ok(Value::Nil)
        }),
        NativeFunction::simple0(sr, "extend", &["self", "xs"], |globals, args, _kwargs| {
            let list = Eval::expect_mutable_list(globals, &args[0])?;
            Eval::extend_from_iterable(globals, &mut list.borrow_mut(), &args[1])?;
            Ok(Value::Nil)
        }),
    ]
    .into_iter()
    .map(|f| (sr.intern_rcstr(f.name()), Value::from(f)))
    .collect();

    let static_methods = vec![NativeFunction::simple0(
        sr,
        "new",
        &["x"],
        |globals, args, _kwargs| Eval::mutable_list_from_iterable(globals, &args[0]),
    )]
    .into_iter()
    .map(|f| (sr.intern_rcstr(f.name()), Value::from(f)))
    .collect();

    Class::new0(
        ClassKind::NativeClass,
        "MutableList".into(),
        vec![base],
        None,
        methods,
        static_methods,
    )
    .into()
}
