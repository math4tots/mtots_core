use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::GeneratorResult;
use crate::NativeFunction;
use crate::NativeIterator;
use crate::Value;
use crate::Symbol;

use std::collections::HashMap;
use std::rc::Rc;

macro_rules! try_ {
    ($eval_result:expr) => {
        match $eval_result {
            Ok(t) => t,
            Err(_) => return GeneratorResult::Error,
        }
    };
}

// There are only two Iterator types, NativeIterator and GeneratorObject
// TODO: Enforce this in the runtime (e.g. something like Scala's sealed traits)

pub(super) fn mkcls(base: Rc<Class>) -> Rc<Class> {
    // While I'm hesitant to add many methods to Iterable because various
    // user classes may implement them, Iterator should have exactly two descendants
    // both of which are builtins. So I'm not really worried about method name conflicts
    // in this case

    let methods = vec![
        NativeFunction::simple0("list", &["self"], |globals, args, _kwargs| {
            Ok(Eval::list_from_iterable(globals, &args[0])?)
        }),
        NativeFunction::simple0("set", &["self"], |globals, args, _kwargs| {
            Ok(Eval::set_from_iterable(globals, &args[0])?)
        }),
        // This really should be two different functions,
        //   - map() => for building a map from an iterable of pairs
        //   - map(f) => for getting a new iterator with f applied to each element
        // TODO: Consider whether this is evil
        NativeFunction::snew(
            "map",
            (&["self"], &[("f", Value::Uninitialized)], None, None),
            |globals, args, _kwargs| {
                if let Value::Uninitialized = &args[1] {
                    Ok(Eval::map_from_iterable(globals, &args[0])?)
                } else {
                    let owner = args[0].clone();
                    let f = args[1].clone();
                    Ok(NativeIterator::new(move |globals, input_value| {
                        match Eval::resume(globals, &owner, input_value) {
                            GeneratorResult::Yield(iterator_yield_value) => {
                                match Eval::call(globals, &f, vec![iterator_yield_value]) {
                                    Ok(new_value) => GeneratorResult::Yield(new_value),
                                    Err(_) => return GeneratorResult::Error,
                                }
                            }
                            result => result,
                        }
                    })
                    .into())
                }
            },
        ),
        NativeFunction::sdnew0(
            "filter",
            &["self", "f"],
            None,
            |_globals, args, _kwargs| {
                let owner = args[0].clone();
                let f = args[1].clone();
                let mut done = false;
                Ok(NativeIterator::new(move |globals, _input_value| {
                    if done {
                        return GeneratorResult::Done(Value::Nil);
                    }
                    loop {
                        if let Some(value) = try_!(Eval::next(globals, &owner)) {
                            let f_result = try_!(Eval::call(globals, &f, vec![value.clone()]));
                            if try_!(Eval::truthy(globals, &f_result)) {
                                return GeneratorResult::Yield(value);
                            }
                        } else {
                            done = true;
                            return GeneratorResult::Done(Value::Nil);
                        }
                    }
                })
                .into())
            },
        ),
        NativeFunction::sdnew0(
            "fold",
            &["self", "acc", "f"],
            None,
            |globals, args, _kwargs| {
                let owner = &args[0];
                let mut acc = args[1].clone();
                let f = &args[2];
                while let Some(x) = Eval::next(globals, owner)? {
                    acc = Eval::call(globals, f, vec![acc, x])?;
                }
                Ok(acc)
            },
        ),
        NativeFunction::sdnew(
            "enumerate",
            (&["self"], &[("start", Value::Int(0))], None, None),
            Some("converts each element x to [i, x] in this iterator"),
            |globals, args, _kwargs| {
                let iterator = args[0].clone();
                let mut i = Eval::expect_int(globals, &args[1])?;
                Ok(NativeIterator::new(move |globals, input_value| {
                    match Eval::resume(globals, &iterator, input_value) {
                        GeneratorResult::Yield(iterator_yield_value) => {
                            let i_val = Value::Int(i);
                            i += 1;
                            GeneratorResult::Yield(vec![i_val, iterator_yield_value].into())
                        }
                        result => result,
                    }
                })
                .into())
            },
        ),
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
                let mut done = false;
                Ok(NativeIterator::new(move |globals, _input_value| {
                    if done {
                        return GeneratorResult::Done(Value::Nil);
                    }
                    let mut current_batch = Vec::new();
                    for iterator in &iterators {
                        match Eval::next(globals, &iterator) {
                            Ok(Some(iterator_yield_value)) => {
                                current_batch.push(iterator_yield_value);
                            }
                            Ok(None) => {
                                done = true;
                                return GeneratorResult::Done(Value::Nil);
                            }
                            Err(_) => {
                                // Let's say if we hit an error, we're done
                                done = true;
                                return GeneratorResult::Error;
                            }
                        }
                    }
                    GeneratorResult::Yield(current_batch.into())
                })
                .into())
            },
        ),
    ]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
    .collect();
    let static_methods = HashMap::new();
    Class::new0(
        ClassKind::Trait,
        "Iterator".into(),
        vec![base],
        None,
        methods,
        static_methods,
    )
    .into()
}
