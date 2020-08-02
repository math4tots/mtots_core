use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::GeneratorResult;
use crate::Globals;
use crate::NativeFunction;
use crate::NativeIterator;
use crate::RcStr;
use crate::Value;
use crate::Symbol;

use std::collections::HashMap;
use std::rc::Rc;

pub(super) fn mkcls(base: Rc<Class>) -> Rc<Class> {
    let methods = vec![
        NativeFunction::simple0("len", &["self"], |globals, args, _kwargs| {
            let s = Eval::expect_string(globals, &args[0])?;
            Ok(Value::Int(s.charlen() as i64))
        }),
        NativeFunction::simple0(
            "starts_with",
            &["self", "prefix"],
            |globals, args, _kwargs| {
                let s = Eval::expect_string(globals, &args[0])?;
                let prefix = Eval::expect_string(globals, &args[1])?;
                let prefix: &str = prefix;
                Ok(s.starts_with(prefix).into())
            },
        ),
        NativeFunction::simple0(
            "ends_with",
            &["self", "suffix"],
            |globals, args, _kwargs| {
                let s = Eval::expect_string(globals, &args[0])?;
                let suffix = Eval::expect_string(globals, &args[1])?;
                let suffix: &str = suffix;
                Ok(s.ends_with(suffix).into())
            },
        ),
        NativeFunction::simple0(
            "lstrip",
            &["self", "prefix"],
            |globals, args, _kwargs| {
                // If self starts with prefix, returns self with prefix removed
                // Otherwise returns self unchanged
                let s = Eval::expect_string(globals, &args[0])?;
                let prefix = Eval::expect_string(globals, &args[1])?;
                let prefix: &str = prefix;
                if s.starts_with(prefix) {
                    let stripped = &s[prefix.len()..];
                    Ok(stripped.into())
                } else {
                    Ok(args[0].clone())
                }
            },
        ),
        NativeFunction::simple0(
            "rstrip",
            &["self", "suffix"],
            |globals, args, _kwargs| {
                // If self ends with suffix, returns self with suffix removed
                // Otherwise returns self unchanged
                let s = Eval::expect_string(globals, &args[0])?;
                let suffix = Eval::expect_string(globals, &args[1])?;
                let suffix: &str = suffix;
                if s.ends_with(suffix) {
                    let stripped = &s[..s.len() - suffix.len()];
                    Ok(stripped.into())
                } else {
                    Ok(args[0].clone())
                }
            },
        ),
        NativeFunction::sdnew0(
            "chars",
            &["self"],
            Some("Returns a list of chars of this String"),
            |globals, args, _kwargs| {
                let s = Eval::expect_string(globals, &args[0])?;
                let mut ret = Vec::new();
                for c in s.chars() {
                    ret.push(globals.char_to_val(c));
                }
                Ok(ret.into())
            },
        ),
        NativeFunction::simple0("trim", &["self"], |globals, args, _kwargs| {
            let s = Eval::expect_string(globals, &args[0])?;
            Ok(s.trim().into())
        }),
        NativeFunction::simple0(
            "replace",
            &["self", "old", "new"],
            |globals, args, _kwargs| {
                let s = Eval::expect_string(globals, &args[0])?;
                let old = Eval::expect_string(globals, &args[1])?;
                let old: &str = old;
                let new = Eval::expect_string(globals, &args[2])?;
                Ok(s.replace(old, new).into())
            },
        ),
        NativeFunction::simple0("split", &["self", "sep"], |globals, args, _kwargs| {
            let string = Eval::expect_string(globals, &args[0])?.clone();
            let sep = Eval::expect_string(globals, &args[1])?.clone();
            Ok(split_by_str(globals, string, sep))
        }),
        NativeFunction::simple0("words", &["self"], |globals, args, _kwargs| {
            let string = Eval::expect_string(globals, &args[0])?.clone();
            Ok(split_by_ws(globals, string))
        }),
        NativeFunction::simple0("lines", &["self"], |globals, args, _kwargs| {
            let string = Eval::expect_string(globals, &args[0])?.clone();
            Ok(split_by_str(globals, string, "\n".into()))
        }),
        NativeFunction::simple0("has", &["self", "pattern"], |globals, args, _kwargs| {
            let string = Eval::expect_string(globals, &args[0])?;
            let pattern = Eval::expect_string(globals, &args[1])?;
            Ok(string.contains(pattern.str()).into())
        }),
        NativeFunction::simple0("join", &["self", "parts"], |globals, args, _kwargs| {
            let mut ret = String::new();
            let sep = Eval::expect_string(globals, &args[0])?;
            let mut first = true;
            for part in Eval::iterable_to_vec(globals, &args[1])? {
                if !first {
                    ret.push_str(sep);
                }
                ret.push_str(Eval::expect_string(globals, &part)?);
                first = false;
            }
            Ok(ret.into())
        }),
        NativeFunction::snew(
            "slice",
            (&["self", "start"], &[("end", Value::Nil)], None, None),
            |globals, args, _kwargs| {
                let (s, _start, _end) =
                    Eval::expect_str_slice(globals, &args[0], &args[1], &args[2])?;
                Ok(s.into())
            },
        ),
        NativeFunction::snew(
            "__slice",
            (&["self", "start", "end"], &[], None, None),
            |globals, args, _kwargs| {
                let (s, _start, _end) =
                    Eval::expect_str_slice(globals, &args[0], &args[1], &args[2])?;
                Ok(s.into())
            },
        ),
        NativeFunction::snew(
            "find",
            (
                &["self", "pattern"],
                &[("start", Value::Nil), ("end", Value::Nil)],
                None,
                None,
            ),
            |globals, args, _kwargs| {
                let pattern = Eval::expect_string(globals, &args[1])?;
                let (s, start, _end) =
                    Eval::expect_str_slice(globals, &args[0], &args[2], &args[3])?;
                Ok(s.find(pattern.str())
                    .map(|i| Value::Int((start + i) as i64))
                    .unwrap_or(Value::Nil))
            },
        ),
        NativeFunction::snew(
            "rfind",
            (
                &["self", "pattern"],
                &[("start", Value::Nil), ("end", Value::Nil)],
                None,
                None,
            ),
            |globals, args, _kwargs| {
                let pattern = Eval::expect_string(globals, &args[1])?;
                let (s, start, _end) =
                    Eval::expect_str_slice(globals, &args[0], &args[2], &args[3])?;
                Ok(s.rfind(pattern.str())
                    .map(|i| Value::Int((start + i) as i64))
                    .unwrap_or(Value::Nil))
            },
        ),
    ]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
    .collect();

    let static_methods = HashMap::new();

    Class::new0(
        ClassKind::NativeClass,
        "String".into(),
        vec![base],
        None,
        methods,
        static_methods,
    )
    .into()
}

// All the split_by_* functions have to be separated because
// the Pattern type is unstable, so I can't even mention the type in
// the signature without a warning

fn split_by_str(_globals: &mut Globals, string: RcStr, sep: RcStr) -> Value {
    let mut i = 0;
    let mut done = false;
    NativeIterator::new(move |_globals, _| {
        let sep: &str = &sep;
        if i < string.len() {
            let s: &str = &string[i..];
            match s.find(sep) {
                Some(j) => {
                    let part = s[..j].to_owned();
                    i += j + sep.len();
                    GeneratorResult::Yield(part.into())
                }
                None => {
                    let part = s.to_owned();
                    i = string.len();
                    done = true;
                    GeneratorResult::Yield(part.into())
                }
            }
        } else if done {
            GeneratorResult::Done(Value::Nil)
        } else {
            done = true;
            GeneratorResult::Yield("".into())
        }
    })
    .into()
}

/// split by whitespace, where runs of whitespace are always clumped into a single separator
/// furthermore, trailing whitespaces are ignored
fn split_by_ws(_globals: &mut Globals, string: RcStr) -> Value {
    let s: &str = &string;
    let mut i = s.find(|c: char| !c.is_whitespace()).unwrap_or(s.len());
    NativeIterator::new(move |_globals, _| {
        if i < string.len() {
            let s: &str = &string[i..];
            match s.find(char::is_whitespace) {
                Some(j) => {
                    let part = s[..j].to_owned();
                    let s = &s[j..];
                    i += j + s.find(|c: char| !c.is_whitespace()).unwrap_or(s.len());
                    GeneratorResult::Yield(part.into())
                }
                None => {
                    // This is the last match
                    let part = s.to_owned();
                    i = string.len();
                    GeneratorResult::Yield(part.into())
                }
            }
        } else {
            GeneratorResult::Done(Value::Nil)
        }
    })
    .into()
}
