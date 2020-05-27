use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::GeneratorResult;
use crate::Globals;
use crate::NativeFunction;
use crate::NativeIterator;
use crate::RcStr;
use crate::SymbolRegistryHandle;
use crate::Value;

use std::collections::HashMap;
use std::rc::Rc;

pub(super) fn mkcls(sr: &SymbolRegistryHandle, base: Rc<Class>) -> Rc<Class> {
    let methods = vec![
        NativeFunction::simple0(sr, "len", &["self"], |globals, args, _kwargs| {
            let s = Eval::expect_string(globals, &args[0])?;
            Ok(Value::Int(s.len() as i64))
        }),
        NativeFunction::simple0(
            sr,
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
            sr,
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
            sr,
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
            sr,
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
        NativeFunction::simple0(
            sr,
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
        NativeFunction::simple0(sr, "split", &["self", "sep"], |globals, args, _kwargs| {
            let string = Eval::expect_string(globals, &args[0])?.clone();
            let sep = Eval::expect_string(globals, &args[1])?.clone();
            Ok(split_by_str(globals, string, sep))
        }),
        NativeFunction::simple0(sr, "words", &["self"], |globals, args, _kwargs| {
            let string = Eval::expect_string(globals, &args[0])?.clone();
            Ok(split_by_ws(globals, string))
        }),
        NativeFunction::simple0(sr, "lines", &["self"], |globals, args, _kwargs| {
            let string = Eval::expect_string(globals, &args[0])?.clone();
            Ok(split_by_str(globals, string, "\n".into()))
        }),
    ]
    .into_iter()
    .map(|f| (sr.intern_rcstr(f.name()), Value::from(f)))
    .collect();

    let static_methods = HashMap::new();

    Class::new0(
        ClassKind::NativeClass,
        "String".into(),
        vec![base],
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