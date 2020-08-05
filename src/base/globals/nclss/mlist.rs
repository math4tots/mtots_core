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
        NativeFunction::simple0("move", &["self"], |globals, args, _kwargs| {
            // Like MutableString:move -- empties out the contents of this List,
            // while returning an immutable List with the same contents
            let list = Eval::expect_mutable_list(globals, &args[0])?;
            let contents = list.replace(vec![]);
            Ok(contents.into())
        }),
        NativeFunction::simple0("clone", &["self"], |globals, args, _kwargs| {
            let list = Eval::expect_mutable_list(globals, &args[0])?;
            let cloned_list = list.borrow().clone();
            Ok(Value::MutableList(RefCell::new(cloned_list).into()))
        }),
        NativeFunction::simple0("len", &["self"], |globals, args, _kwargs| {
            let list = Eval::expect_mutable_list(globals, &args[0])?;
            Ok(Value::Int(list.borrow().len() as i64))
        }),
        NativeFunction::simple0("__getitem", &["self", "i"], |globals, args, _kwargs| {
            let list = Eval::expect_mutable_list(globals, &args[0])?;
            let i = Eval::expect_index(globals, &args[1], list.borrow().len())?;
            Ok((list.borrow())[i].clone())
        }),
        NativeFunction::simple0(
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
        NativeFunction::sdnew0(
            "__slice",
            &["self", "start", "end"],
            Some("Creates a new mutable list consisting of a subrange of this mutable list"),
            |globals, args, _kwargs| {
                let list = Eval::expect_mutable_list(globals, &args[0])?;
                let list = list.borrow();
                let (start, end) =
                    Eval::expect_range_indices(globals, &args[1], &args[2], list.len())?;
                Ok(Value::MutableList(
                    RefCell::new((*list)[start..end].to_vec()).into(),
                ))
            },
        ),
        NativeFunction::simple0("map", &["self", "f"], |globals, args, _kwargs| {
            let list = Eval::expect_mutable_list(globals, &args[0])?;
            let f = &args[1];
            let mut ret = Vec::new();
            for x in list.borrow().iter() {
                let y = Eval::call(globals, f, vec![x.clone()])?;
                ret.push(y);
            }
            Ok(Value::MutableList(RefCell::new(ret).into()))
        }),
        NativeFunction::sdnew(
            "remove",
            (
                &["self", "i"],
                &[("default", Value::Uninitialized)],
                None,
                None,
            ),
            Some("Removes and returns the element at position i"),
            |globals, args, _kwargs| {
                let list = Eval::expect_mutable_list(globals, &args[0])?;
                let try_index = Eval::try_index(&args[1], list.borrow().len());
                match try_index {
                    Some(i) => Ok(list.borrow_mut().remove(i)),
                    None => {
                        if let Value::Uninitialized = &args[2] {
                            Eval::expect_index(globals, &args[1], list.borrow().len())?;
                            panic!("try_index failed, but expect_index succeeded");
                        } else {
                            Ok(args[2].clone())
                        }
                    }
                }
            },
        ),
        NativeFunction::sdnew0(
            "insert",
            &["self", "i", "value"],
            Some("Removes and returns the element at position i"),
            |globals, args, _kwargs| {
                let list = Eval::expect_mutable_list(globals, &args[0])?;
                let len = list.borrow().len();
                let i = Eval::expect_index(globals, &args[1], len)?;
                list.borrow_mut().insert(i, args[2].clone());
                Ok(Value::Nil)
            },
        ),
        NativeFunction::sdnew0(
            "splice",
            &["self", "start", "end", "iterable"],
            Some("Removes and returns the element at position i"),
            |globals, args, _kwargs| {
                let list = Eval::expect_mutable_list(globals, &args[0])?;
                let len = list.borrow().len();
                let (start, end) = Eval::expect_range_indices(globals, &args[1], &args[2], len)?;
                let vec = Eval::iterable_to_vec(globals, &args[3])?;
                let ret: Vec<_> = list.borrow_mut().splice(start..end, vec).collect();
                Ok(ret.into())
            },
        ),
        NativeFunction::simple0("pop", &["self"], |globals, args, _kwargs| {
            let list = Eval::expect_mutable_list(globals, &args[0])?;
            match list.borrow_mut().pop() {
                Some(value) => Ok(value),
                None => return globals.set_empty_pop_error()?,
            }
        }),
        NativeFunction::simple0("push", &["self", "x"], |globals, args, _kwargs| {
            let list = Eval::expect_mutable_list(globals, &args[0])?;
            let item = args[1].clone();
            list.borrow_mut().push(item);
            Ok(Value::Nil)
        }),
        NativeFunction::simple0("extend", &["self", "xs"], |globals, args, _kwargs| {
            let list = Eval::expect_mutable_list(globals, &args[0])?;
            Eval::extend_from_iterable(globals, &mut list.borrow_mut(), &args[1])?;
            Ok(Value::Nil)
        }),
        NativeFunction::simple0("reverse", &["self"], |globals, args, _kwargs| {
            let list = Eval::expect_mutable_list(globals, &args[0])?;
            list.borrow_mut().reverse();
            Ok(Value::Nil)
        }),
        NativeFunction::simple0("resize", &["self", "new_size"], |globals, args, _kwargs| {
            let list = Eval::expect_mutable_list(globals, &args[0])?;
            let mut list = list.borrow_mut();
            let new_size = Eval::expect_usize(globals, &args[1])?;
            if list.len() < new_size {
                for _ in list.len()..new_size {
                    list.push(Value::Nil);
                }
            } else {
                list.truncate(new_size);
            }
            Ok(Value::Nil)
        }),
        NativeFunction::snew(
            "any",
            (&["self"], &[("f", Value::Nil)], None, None),
            |globals, args, _kwargs| {
                let list = Eval::expect_mutable_list(globals, &args[0])?;
                let f = &args[1];
                for x in list.borrow().clone() {
                    let truthy = if let Value::Nil = f {
                        Eval::truthy(globals, &x)?
                    } else {
                        let fx = Eval::call(globals, f, vec![x])?;
                        Eval::truthy(globals, &fx)?
                    };
                    if truthy {
                        return Ok(true.into());
                    }
                }
                Ok(false.into())
            },
        ),
        NativeFunction::snew(
            "all",
            (&["self"], &[("f", Value::Nil)], None, None),
            |globals, args, _kwargs| {
                let list = Eval::expect_mutable_list(globals, &args[0])?;
                let f = &args[1];
                for x in list.borrow().clone() {
                    let truthy = if let Value::Nil = f {
                        Eval::truthy(globals, &x)?
                    } else {
                        let fx = Eval::call(globals, f, vec![x])?;
                        Eval::truthy(globals, &fx)?
                    };
                    if !truthy {
                        return Ok(false.into());
                    }
                }
                Ok(true.into())
            },
        ),
    ]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
    .collect();

    let static_methods = vec![
        NativeFunction::simple0("__call", &["x"], |globals, args, _kwargs| {
            Eval::mutable_list_from_iterable(globals, &args[0])
        }),
        NativeFunction::simple0("from_iterable", &["x"], |globals, args, _kwargs| {
            Eval::mutable_list_from_iterable(globals, &args[0])
        }),
    ]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
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
