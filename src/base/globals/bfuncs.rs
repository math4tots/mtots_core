use crate::Class;
use crate::Eval;
use crate::NativeFunction;
use crate::SymbolRegistryHandle;
use crate::Value;

use std::rc::Rc;

pub struct NativeFunctions {
    print: Rc<NativeFunction>,
    str_: Rc<NativeFunction>,
    repr: Rc<NativeFunction>,
    type_: Rc<NativeFunction>,
    sorted: Rc<NativeFunction>,
    min: Rc<NativeFunction>,
    max: Rc<NativeFunction>,
    assert: Rc<NativeFunction>,
    assert_eq: Rc<NativeFunction>,
    assert_raises: Rc<NativeFunction>,
    dunder_import: Rc<NativeFunction>,
    dunder_malloc: Rc<NativeFunction>,
    dunder_args: Rc<NativeFunction>,
    dunder_main: Rc<NativeFunction>,
    dunder_raise: Rc<NativeFunction>,
    dunder_try: Rc<NativeFunction>,
    dunder_iter: Rc<NativeFunction>,
}

impl NativeFunctions {
    pub fn for_builtins(&self) -> Vec<&Rc<NativeFunction>> {
        vec![
            &self.print,
            &self.str_,
            &self.repr,
            &self.type_,
            &self.sorted,
            &self.min,
            &self.max,
            &self.assert,
            &self.assert_eq,
            &self.assert_raises,
            &self.dunder_import,
            &self.dunder_malloc,
            &self.dunder_args,
            &self.dunder_main,
            &self.dunder_raise,
            &self.dunder_try,
            &self.dunder_iter,
        ]
    }
}

pub(super) fn new(sr: &SymbolRegistryHandle) -> NativeFunctions {
    let print = NativeFunction::simple0(sr, "print", &["x"], |globals, args, _kwargs| {
        let s = Eval::str(globals, &args[0])?;
        println!("{}", s);
        Ok(Value::Nil)
    })
    .into();

    let str_ = NativeFunction::simple0(sr, "str", &["x"], |globals, args, _kwargs| {
        Ok(Value::String(Eval::str(globals, &args[0])?))
    })
    .into();

    let repr = NativeFunction::simple0(sr, "repr", &["x"], |globals, args, _kwargs| {
        Ok(Value::String(Eval::repr(globals, &args[0])?))
    })
    .into();

    let type_ = NativeFunction::simple0(sr, "type", &["x"], |globals, args, _kwargs| {
        Ok(Eval::classof(globals, &args[0])?.clone().into())
    })
    .into();

    let sorted = NativeFunction::simple0(sr, "sorted", &["xs"], |globals, args, _kwargs| {
        let mut vec = Eval::iterable_to_vec(globals, &args[0])?;
        Eval::sort(globals, &mut vec)?;
        Ok(vec.into())
    })
    .into();

    let min = NativeFunction::snew(sr, "min", (&["xs"], &[], Some("varargs"), None), |globals, args, _kwargs| {
        let iterator = if args.len() == 1 {
            Eval::iter(globals, &args[0])?
        } else {
            Eval::iter(globals, &args.into())?
        };
        let mut best = match Eval::next(globals, &iterator)? {
            Some(first) => first,
            None => return globals.set_exc_str("Empty iterable passed to min"),
        };
        while let Some(x) = Eval::next(globals, &iterator)? {
            if Eval::lt(globals, &x, &best)? {
                best = x;
            }
        }
        Ok(best)
    })
    .into();

    let max = NativeFunction::snew(sr, "max", (&["xs"], &[], Some("varargs"), None), |globals, args, _kwargs| {
        let iterator = if args.len() == 1 {
            Eval::iter(globals, &args[0])?
        } else {
            Eval::iter(globals, &args.into())?
        };
        let mut best = match Eval::next(globals, &iterator)? {
            Some(first) => first,
            None => return globals.set_exc_str("Empty iterable passed to max"),
        };
        while let Some(x) = Eval::next(globals, &iterator)? {
            if Eval::lt(globals, &best, &x)? {
                best = x;
            }
        }
        Ok(best)
    })
    .into();

    let assert = NativeFunction::simple0(sr, "assert", &["x"], |globals, args, _kwargs| {
        if !Eval::truthy(globals, &args[0])? {
            return globals.set_assert_error(&format!("Assertion failed").into());
        }
        Ok(Value::Nil)
    })
    .into();

    let assert_eq =
        NativeFunction::simple0(sr, "assert_eq", &["a", "b"], |globals, args, _kwargs| {
            if !Eval::eq(globals, &args[0], &args[1])? {
                let str1 = Eval::repr(globals, &args[0])?;
                let str2 = Eval::repr(globals, &args[1])?;
                return globals
                    .set_assert_error(&format!("Expected {} to equal {}", str1, str2).into());
            }
            Ok(Value::Nil)
        })
        .into();

    let assert_raises = NativeFunction::simple0(
        sr,
        "assert_raises",
        &["exck", "f"],
        |globals, args, _kwargs| {
            let exception_kind = Eval::expect_exception_kind(globals, &args[0])?;
            let trace_len = globals.trace_len();
            match Eval::call(globals, &args[1], vec![]) {
                Ok(_) => globals.set_assert_error(
                    &format!("Expected {} to be raised", exception_kind.name()).into(),
                ),
                Err(_) => {
                    let exc = globals.exc_move();
                    if exc.matches(exception_kind) {
                        // success~
                        globals.trace_trunc(trace_len);
                        Ok(Value::Nil)
                    } else {
                        // if the exeception doesn't match, just pass it along
                        globals.set_exc(exc)
                    }
                }
            }
        },
    )
    .into();

    let dunder_import =
        NativeFunction::simple0(sr, "__import", &["name"], |globals, args, _kwargs| {
            let name = Eval::expect_symbollike(globals, &args[0])?;
            let module = globals.load_by_symbol(name)?;
            Ok(Value::Module(module))
        })
        .into();

    let dunder_malloc = NativeFunction::simple0(
        sr,
        "__malloc",
        &["cls", "fields"],
        |globals, args, _kwargs| {
            let cls = Eval::expect_class(globals, &args[0])?;
            let fields = Eval::expect_list(globals, &args[1])?;
            let obj = Class::instantiate(cls, globals, fields.clone(), None)?;
            Ok(obj.into())
        },
    )
    .into();

    let dunder_args = NativeFunction::simple0(sr, "__args", &[], |globals, _args, _kwargs| {
        let mut ret: Vec<Value> = Vec::new();
        for arg in globals.cli_args() {
            ret.push(arg.clone().into());
        }
        Ok(ret.into())
    })
    .into();

    let dunder_main = NativeFunction::simple0(sr, "__main", &[], |globals, _args, _kwargs| {
        match globals.main_module_name() {
            Some(main_module_name) => Ok(main_module_name.clone().into()),
            None => Ok(Value::Nil),
        }
    })
    .into();

    let dunder_raise =
        NativeFunction::simple0(sr, "__raise", &["exc"], |globals, args, _kwargs| {
            let exc = Eval::move_exc(globals, args.into_iter().next().unwrap())?;
            globals.set_exc(exc)
        })
        .into();

    let dunder_try = NativeFunction::snew(
        sr,
        "__try",
        (&["main"], &[], Some("rest"), None),
        |globals, mut args, _kwargs| {
            let finally_clause = if args.len() % 2 == 0 {
                Some(args.pop().unwrap())
            } else {
                None
            };

            let mut args = args.into_iter();

            let result = Eval::call(globals, &args.next().unwrap(), vec![]);

            let saved_trace_len = globals.trace_len();
            match result {
                Ok(value) => {
                    if let Some(finally_clause) = finally_clause {
                        Eval::call(globals, &finally_clause, vec![])?;
                    }
                    Ok(value)
                }
                Err(_indicator) => {
                    // We need to be careful here to make sure trace is handled correctly
                    // We need to first check if any of the listed exception kinds match.
                    // If none matches, we don't want to touch the trace, so we don't
                    // truncate the stack at the beginning.
                    // On the other hand, we want to make sure not to do anything that
                    // might need to touch the stack while we're checking for if any
                    // exception kind matches.
                    let exception = globals.exc_move();
                    while let Some(exc_kind) = args.next() {
                        let exc_kind = match exc_kind {
                            Value::ExceptionKind(exc_kind) => exc_kind,
                            exc_kind => {
                                globals.trace_trunc(saved_trace_len);
                                Eval::expect_exception_kind(globals, &exc_kind)?;
                                panic!("Exception should have been thrown")
                            }
                        };
                        let body = args.next().unwrap();
                        if exception.matches(&exc_kind) {
                            globals.trace_trunc(saved_trace_len);
                            let result = Eval::call(
                                globals,
                                &body,
                                vec![Value::Exception(exception.into())],
                            );
                            if let Some(finally_clause) = finally_clause {
                                Eval::call(globals, &finally_clause, vec![])?;
                            }
                            return result;
                        }
                    }

                    // If no match is found, pass the exception back up the stack...
                    if let Some(finally_clause) = finally_clause {
                        // unfortunately, when we have a finally clause, we'll have to
                        // lose a part of the trace (for now)
                        globals.trace_trunc(saved_trace_len);
                        Eval::call(globals, &finally_clause, vec![])?;
                    }
                    globals.set_exc(exception)
                }
            }
        },
    )
    .into();

    let dunder_iter =
        NativeFunction::simple0(sr, "__iter", &["iterable"], |globals, args, _kwargs| {
            Eval::iter(globals, &args[0])
        })
        .into();

    NativeFunctions {
        print,
        str_,
        repr,
        type_,
        sorted,
        min,
        max,
        assert,
        assert_eq,
        assert_raises,
        dunder_import,
        dunder_malloc,
        dunder_args,
        dunder_main,
        dunder_raise,
        dunder_try,
        dunder_iter,
    }
}
