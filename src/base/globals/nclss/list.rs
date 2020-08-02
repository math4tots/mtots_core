use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::NativeFunction;
use crate::Value;
use crate::Symbol;

use std::rc::Rc;

pub(super) fn mkcls(base: Rc<Class>) -> Rc<Class> {
    let methods = vec![
        NativeFunction::simple0("len", &["self"], |globals, args, _kwargs| {
            let list = Eval::expect_list(globals, &args[0])?;
            Ok(Value::Int(list.len() as i64))
        }),
        NativeFunction::simple0("__getitem", &["self", "i"], |globals, args, _kwargs| {
            let list = Eval::expect_list(globals, &args[0])?;
            let i = Eval::expect_index(globals, &args[1], list.len())?;
            Ok(list[i].clone())
        }),
        NativeFunction::simple0("map", &["self", "f"], |globals, args, _kwargs| {
            let list = Eval::expect_list(globals, &args[0])?;
            let f = &args[1];
            let mut ret = Vec::new();
            for x in list {
                let y = Eval::call(globals, f, vec![x.clone()])?;
                ret.push(y);
            }
            Ok(ret.into())
        }),
        NativeFunction::simple0("filter", &["self", "f"], |globals, args, _kwargs| {
            let list = Eval::expect_list(globals, &args[0])?;
            let f = &args[1];
            let mut ret = Vec::new();
            for x in list {
                let xp = Eval::call(globals, f, vec![x.clone()])?;
                let truthy = Eval::truthy(globals, &xp)?;
                if truthy {
                    ret.push(x.clone());
                }
            }
            Ok(ret.into())
        }),
        NativeFunction::simple0("reversed", &["self"], |globals, args, _kwargs| {
            let mut list = Eval::move_or_clone_list(globals, args.into_iter().next().unwrap())?;
            list.reverse();
            Ok(list.into())
        }),
        NativeFunction::sdnew(
            "zip",
            (&["self"], &[], Some("iterables"), None),
            None,
            |globals, args, _kwargs| {
                let iterators = {
                    let mut vec = Vec::new();
                    for arg in args {
                        vec.push(Eval::iter(globals, &arg)?);
                    }
                    vec
                };
                let mut ret = Vec::new();
                loop {
                    let mut current_batch = Vec::new();
                    for iterator in &iterators {
                        if let Some(value) = Eval::next(globals, iterator)? {
                            current_batch.push(value);
                        } else {
                            return Ok(ret.into());
                        }
                    }
                    ret.push(Value::List(current_batch.into()));
                }
            },
        ),
        NativeFunction::snew(
            "any",
            (&["self"], &[("f", Value::Nil)], None, None),
            |globals, args, _kwargs| {
                let list = Eval::expect_list(globals, &args[0])?;
                let f = &args[1];
                for x in list {
                    let truthy = if let Value::Nil = f {
                        Eval::truthy(globals, x)?
                    } else {
                        let fx = Eval::call(globals, f, vec![x.clone()])?;
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
                let list = Eval::expect_list(globals, &args[0])?;
                let f = &args[1];
                for x in list {
                    let truthy = if let Value::Nil = f {
                        Eval::truthy(globals, x)?
                    } else {
                        let fx = Eval::call(globals, f, vec![x.clone()])?;
                        Eval::truthy(globals, &fx)?
                    };
                    if !truthy {
                        return Ok(false.into());
                    }
                }
                Ok(true.into())
            },
        ),
        NativeFunction::simple0("iter", &["self"], |globals, args, _kwargs| {
            Eval::iter(globals, &args[0])
        }),
        NativeFunction::simple0("has", &["self", "x"], |globals, args, _kwargs| {
            let list = Eval::expect_list(globals, &args[0])?;
            let x = &args[1];
            for y in list.iter() {
                if Eval::eq(globals, x, y)? {
                    return Ok(true.into());
                }
            }
            Ok(false.into())
        }),
        NativeFunction::sdnew0(
            "__slice",
            &["self", "start", "end"],
            Some("Creates a new list consisting of a subrange of this list"),
            |globals, args, _kwargs| {
                let list = Eval::expect_list(globals, &args[0])?;
                let (start, end) =
                    Eval::expect_range_indices(globals, &args[1], &args[2], list.len())?;
                Ok((*list)[start..end].to_vec().into())
            },
        ),
    ]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
    .collect();

    let static_methods = vec![
        NativeFunction::simple0("__call", &["iterable"], |globals, args, _kwargs| {
            Eval::list_from_iterable(globals, &args[0])
        }),
        NativeFunction::simple0(
            "from_iterable",
            &["iterable"],
            |globals, args, _kwargs| Eval::list_from_iterable(globals, &args[0]),
        ),
    ]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
    .collect();

    Class::new0(
        ClassKind::NativeClass,
        "List".into(),
        vec![base],
        None,
        methods,
        static_methods,
    )
    .into()
}
