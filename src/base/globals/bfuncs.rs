use crate::Class;
use crate::Eval;
use crate::NativeFunction;
use crate::ParameterInfo;
use crate::Value;

use std::rc::Rc;

pub struct NativeFunctions {
    print: Rc<NativeFunction>,
    int: Rc<NativeFunction>,
    float: Rc<NativeFunction>,
    str_: Rc<NativeFunction>,
    repr: Rc<NativeFunction>,
    type_: Rc<NativeFunction>,
    sorted: Rc<NativeFunction>,
    min: Rc<NativeFunction>,
    max: Rc<NativeFunction>,
    ord: Rc<NativeFunction>,
    chr: Rc<NativeFunction>,
    hash: Rc<NativeFunction>,
    assert: Rc<NativeFunction>,
    assert_eq: Rc<NativeFunction>,
    assert_raises: Rc<NativeFunction>,
    dunder_import: Rc<NativeFunction>,
    dunder_malloc: Rc<NativeFunction>,
    dunder_new: Rc<NativeFunction>,
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
            &self.int,
            &self.float,
            &self.str_,
            &self.repr,
            &self.type_,
            &self.sorted,
            &self.min,
            &self.max,
            &self.ord,
            &self.chr,
            &self.hash,
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

    pub(crate) fn dunder_new(&self) -> &Rc<NativeFunction> {
        &self.dunder_new
    }
}

pub(super) fn new() -> NativeFunctions {
    let print = NativeFunction::new("print", ["x"], None, |globals, args, _kwargs| {
        let s = Eval::str(globals, &args[0])?;
        println!("{}", s);
        Ok(Value::Nil)
    })
    .into();

    let int = NativeFunction::new("int", ["x"], None, |globals, args, _kwargs| {
        let i = match args.into_iter().next().unwrap() {
            Value::Number(n) => n as i64,
            Value::String(string) => globals.converr(string.parse::<i64>())?,
            x => Eval::expect_int(globals, &x)?,
        };
        Ok(Value::from(i))
    })
    .into();

    let float = NativeFunction::new("float", ["x"], None, |globals, args, _kwargs| {
        let i = match args.into_iter().next().unwrap() {
            Value::Number(f) => f,
            Value::String(string) => globals.converr(string.parse::<f64>())?,
            arg => Eval::expect_floatlike(globals, &arg)?,
        };
        Ok(Value::from(i))
    })
    .into();

    let str_ = NativeFunction::new("str", ["x"], None, |globals, args, _kwargs| {
        Ok(Value::String(Eval::str(globals, &args[0])?))
    })
    .into();

    let repr = NativeFunction::new("repr", ["x"], None, |globals, args, _kwargs| {
        Ok(Value::String(Eval::repr(globals, &args[0])?))
    })
    .into();

    let type_ = NativeFunction::new("type", ["x"], None, |globals, args, _kwargs| {
        Ok(Eval::classof(globals, &args[0])?.clone().into())
    })
    .into();

    let sorted = NativeFunction::new("sorted", &["xs"], None, |globals, args, _kwargs| {
        let mut vec = Eval::iterable_to_vec(globals, &args[0])?;
        Eval::sort(globals, &mut vec)?;
        Ok(vec.into())
    })
    .into();

    let min = NativeFunction::new(
        "min",
        ParameterInfo::builder().required("xs").variadic("varargs"),
        concat!(
            "Gets the minimum of an iterable\n",
            "If exactly one argument is provided, it is assumed to be an iterable, ",
            "and this function returns the minimum element in that iterable.\n",
            "This function with throw if the iterable is empty.\n",
            "If more than one argument is provided, the list of all arguments is ",
            "taken as the iterable, and the minimum element is returned.\n",
        ),
        |globals, args, _kwargs| {
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
        },
    )
    .into();

    let max = NativeFunction::new(
        "max",
        ParameterInfo::builder().required("xs").variadic("varargs"),
        concat!(
            "Gets the maximum of an iterable\n",
            "If exactly one argument is provided, it is assumed to be an iterable, ",
            "and this function returns the maximum element in that iterable.\n",
            "This function with throw if the iterable is empty.\n",
            "If more than one argument is provided, the list of all arguments is ",
            "taken as the iterable, and the maximum element is returned.\n",
        ),
        |globals, args, _kwargs| {
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
        },
    )
    .into();

    let ord = NativeFunction::new("ord", &["ch"], None, |globals, args, _kwargs| {
        let chstr = Eval::expect_string(globals, &args[0])?;
        let chars: Vec<_> = chstr.chars().collect();
        if chars.len() != 1 {
            return globals.set_exc_str(&format!(
                "Expected string with exactly 1 char, but got {} chars",
                chars.len()
            ));
        }
        Ok((chars[0] as u32).into())
    })
    .into();

    let chr = NativeFunction::new("chr", &["i"], None, |globals, args, _kwargs| {
        let i = Eval::expect_int(globals, &args[0])?;
        let i = Eval::check_u32(globals, i)?;
        match std::char::from_u32(i) {
            Some(ch) => Ok(globals.char_to_val(ch)),
            None => globals.set_exc_str(&format!("{} is not a valid unicode codepoint", i,)),
        }
    })
    .into();

    let hash = NativeFunction::new("hash", ["x"], None, |globals, args, _kwargs| {
        let hash = Eval::hash(globals, &args[0])?;
        Ok(Value::from(hash))
    })
    .into();

    let assert = NativeFunction::new("assert", ["x"], None, |globals, args, _kwargs| {
        if !Eval::truthy(globals, &args[0])? {
            return globals.set_assert_error(&format!("Assertion failed").into());
        }
        Ok(Value::Nil)
    })
    .into();

    let assert_eq =
        NativeFunction::new("assert_eq", &["a", "b"], None, |globals, args, _kwargs| {
            if !Eval::eq(globals, &args[0], &args[1])? {
                let str1 = Eval::repr(globals, &args[0])?;
                let str2 = Eval::repr(globals, &args[1])?;
                return globals
                    .set_assert_error(&format!("Expected {} to equal {}", str1, str2).into());
            }
            Ok(Value::Nil)
        })
        .into();

    let assert_raises = NativeFunction::new(
        "assert_raises",
        &["exck", "f"],
        None,
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
        NativeFunction::new("__import", &["name"], None, |globals, args, _kwargs| {
            let name = Eval::expect_symbollike(globals, &args[0])?;
            let module = globals.load_by_symbol(name)?;
            Ok(Value::Module(module))
        })
        .into();

    let dunder_malloc = NativeFunction::new(
        "__malloc",
        &["cls", "fields"],
        None,
        |globals, args, _kwargs| {
            let cls = Eval::expect_class(globals, &args[0])?;
            let fields = Eval::expect_list(globals, &args[1])?;
            let obj = Class::instantiate(cls, globals, fields.clone(), None)?;
            Ok(obj.into())
        },
    )
    .into();

    let dunder_new = NativeFunction::new(
        "__new",
        ParameterInfo::builder()
            .required("class")
            .variadic("args")
            .keywords("kwargs"),
        Some(
            concat!(
                "Function used by 'new' expressions when building new instances\n",
                "This function should never be called directly"
            )
            .into(),
        ),
        |globals, mut args, kwargs| {
            // Making the first parameter of the function 'class' is intentional
            // since the user may pass arbitrary keyword parameters, a normal
            // identifier may clash with the keyword parameter.
            // The user can provide a kwargs table that contains class as a key,
            // however, it should never conflict with a class field, since fields
            // should all be legal identifiers.
            let cls_val = args.remove(0);
            let cls = Eval::expect_class(globals, &cls_val)?;
            let obj = Class::instantiate(cls, globals, args, kwargs)?;
            Ok(obj.into())
        },
    )
    .into();

    let dunder_args = NativeFunction::new("__args", &[], None, |globals, _args, _kwargs| {
        let mut ret: Vec<Value> = Vec::new();
        for arg in globals.cli_args() {
            ret.push(arg.clone().into());
        }
        Ok(ret.into())
    })
    .into();

    let dunder_main =
        NativeFunction::new("__main", &[], None, |globals, _args, _kwargs| match globals
            .main_module_name()
        {
            Some(main_module_name) => Ok(main_module_name.clone().into()),
            None => Ok(Value::Nil),
        })
        .into();

    let dunder_raise = NativeFunction::new("__raise", &["exc"], None, |globals, args, _kwargs| {
        let exc = Eval::move_exc(globals, args.into_iter().next().unwrap())?;
        globals.set_exc(exc)
    })
    .into();

    let dunder_try = NativeFunction::new(
        "__try",
        ParameterInfo::builder().required("main").variadic("rest"),
        None,
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
        NativeFunction::new("__iter", &["iterable"], None, |globals, args, _kwargs| {
            Eval::iter(globals, &args[0])
        })
        .into();

    NativeFunctions {
        print,
        int,
        float,
        str_,
        repr,
        type_,
        sorted,
        min,
        max,
        ord,
        chr,
        hash,
        assert,
        assert_eq,
        assert_raises,
        dunder_import,
        dunder_malloc,
        dunder_new,
        dunder_args,
        dunder_main,
        dunder_raise,
        dunder_try,
        dunder_iter,
    }
}
