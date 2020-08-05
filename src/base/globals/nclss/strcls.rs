use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::GeneratorResult;
use crate::Globals;
use crate::NativeFunction;
use crate::NativeIterator;
use crate::ParameterInfo;
use crate::RcStr;
use crate::Symbol;
use crate::Value;

use std::collections::HashMap;
use std::rc::Rc;

pub(super) fn mkcls(base: Rc<Class>) -> Rc<Class> {
    let methods = vec![
        NativeFunction::new("len", &["self"], None, |globals, args, _kwargs| {
            let s = Eval::expect_string(globals, &args[0])?;
            Ok(Value::Int(s.charlen() as i64))
        }),
        NativeFunction::new(
            "starts_with",
            &["self", "prefix"],
            None,
            |globals, args, _kwargs| {
                let s = Eval::expect_string(globals, &args[0])?;
                let prefix = Eval::expect_string(globals, &args[1])?;
                let prefix: &str = prefix;
                Ok(s.starts_with(prefix).into())
            },
        ),
        NativeFunction::new(
            "ends_with",
            &["self", "suffix"],
            None,
            |globals, args, _kwargs| {
                let s = Eval::expect_string(globals, &args[0])?;
                let suffix = Eval::expect_string(globals, &args[1])?;
                let suffix: &str = suffix;
                Ok(s.ends_with(suffix).into())
            },
        ),
        NativeFunction::new(
            "lstrip",
            &["self", "prefix"],
            None,
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
        NativeFunction::new(
            "rstrip",
            &["self", "suffix"],
            None,
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
        NativeFunction::new(
            "chars",
            &["self"],
            "Returns a list of chars of this String",
            |globals, args, _kwargs| {
                let s = Eval::expect_string(globals, &args[0])?;
                let mut ret = Vec::new();
                for c in s.chars() {
                    ret.push(globals.char_to_val(c));
                }
                Ok(ret.into())
            },
        ),
        NativeFunction::new("trim", &["self"], None, |globals, args, _kwargs| {
            let s = Eval::expect_string(globals, &args[0])?;
            Ok(s.trim().into())
        }),
        NativeFunction::new(
            "replace",
            &["self", "old", "new"],
            None,
            |globals, args, _kwargs| {
                let s = Eval::expect_string(globals, &args[0])?;
                let old = Eval::expect_string(globals, &args[1])?;
                let old: &str = old;
                let new = Eval::expect_string(globals, &args[2])?;
                Ok(s.replace(old, new).into())
            },
        ),
        NativeFunction::new("split", &["self", "sep"], None, |globals, args, _kwargs| {
            let string = Eval::expect_string(globals, &args[0])?.clone();
            let sep = Eval::expect_string(globals, &args[1])?.clone();
            Ok(split_by_str(globals, string, sep))
        }),
        NativeFunction::new("words", &["self"], None, |globals, args, _kwargs| {
            let string = Eval::expect_string(globals, &args[0])?.clone();
            Ok(split_by_ws(globals, string))
        }),
        NativeFunction::new("lines", &["self"], None, |globals, args, _kwargs| {
            let string = Eval::expect_string(globals, &args[0])?.clone();
            Ok(split_by_str(globals, string, "\n".into()))
        }),
        NativeFunction::new(
            "has",
            &["self", "pattern"],
            None,
            |globals, args, _kwargs| {
                let string = Eval::expect_string(globals, &args[0])?;
                let pattern = Eval::expect_string(globals, &args[1])?;
                Ok(string.contains(pattern.str()).into())
            },
        ),
        NativeFunction::new(
            "join",
            &["self", "parts"],
            None,
            |globals, args, _kwargs| {
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
            },
        ),
        NativeFunction::new(
            "slice",
            ParameterInfo::builder()
                .required("self")
                .required("start")
                .optional("end", ()),
            None,
            |globals, args, _kwargs| {
                let (s, _start, _end) =
                    Eval::expect_str_slice(globals, &args[0], &args[1], &args[2])?;
                Ok(s.into())
            },
        ),
        NativeFunction::new(
            "__slice",
            ["self", "start", "end"],
            None,
            |globals, args, _kwargs| {
                let (s, _start, _end) =
                    Eval::expect_str_slice(globals, &args[0], &args[1], &args[2])?;
                Ok(s.into())
            },
        ),
        NativeFunction::new(
            "find",
            ParameterInfo::builder()
                .required("self")
                .required("pattern")
                .optional("start", ())
                .optional("end", ()),
            None,
            |globals, args, _kwargs| {
                let pattern = Eval::expect_string(globals, &args[1])?;
                let (s, start, _end) =
                    Eval::expect_str_slice(globals, &args[0], &args[2], &args[3])?;
                Ok(s.find(pattern.str())
                    .map(|i| Value::Int((start + i) as i64))
                    .unwrap_or(Value::Nil))
            },
        ),
        NativeFunction::new(
            "rfind",
            ParameterInfo::builder()
                .required("self")
                .required("pattern")
                .optional("start", ())
                .optional("end", ()),
            None,
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
