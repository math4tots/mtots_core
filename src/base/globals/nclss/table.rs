use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::NativeFunction;
use crate::Symbol;
use crate::Table;
use crate::Value;

use std::rc::Rc;

pub(super) fn mkcls(base: Rc<Class>) -> Rc<Class> {
    let methods = vec![
        NativeFunction::simple0("len", &["self"], |globals, args, _kwargs| {
            let s = Eval::expect_table(globals, &args[0])?;
            Ok(Value::Int(s.len() as i64))
        }),
        NativeFunction::snew(
            "keys",
            (&["self"], &[], None, None),
            |globals, args, _kwargs| {
                // returns a sorted list of all keys of the table
                // We don't allow iterating over a table, because for Table
                // objects I want to keep using the HashMap in the standard library
                // and those come with a non-static lifetime parameter that's
                // annoying to translate to script-land. If an iterator is really
                // needed, the Map type is available.
                let table = Eval::expect_table(globals, &args[0])?;
                let mut keys = Vec::new();
                for (key, _) in table.iter() {
                    keys.push(*key);
                }
                keys.sort_by(|a, b| a.str().cmp(b.str()));
                let keys: Vec<_> = keys.into_iter().map(Value::Symbol).collect();
                Ok(keys.into())
            },
        ),
        NativeFunction::snew(
            "get",
            (
                &["self", "key"],
                &[("default", Value::Uninitialized)],
                None,
                None,
            ),
            |globals, args, _kwargs| {
                // Look up the entry in a Table by symbol determined at runtime
                let table = Eval::expect_table(globals, &args[0])?;
                let key = Eval::expect_symbol(globals, &args[1])?;
                match table.get(key) {
                    Some(value) => Ok(value.clone()),
                    None => {
                        if let Value::Uninitialized = &args[2] {
                            globals.set_exc_str(&format!("Key {:?} not found in this Table", key,))
                        } else {
                            Ok(args[2].clone())
                        }
                    }
                }
            },
        ),
        NativeFunction::snew(
            "add",
            (&["self"], &[], None, Some("kwargs")),
            |globals, args, kwargs| {
                let mut table =
                    Eval::move_or_clone_table(globals, args.into_iter().next().unwrap())?
                        .map_move();
                for (key, val) in kwargs.unwrap() {
                    table.insert(key, val);
                }
                Ok(Value::Table(Table::new(table).into()))
            },
        ),
        NativeFunction::snew(
            "merge",
            (&["self"], &[], Some("args"), None),
            |globals, args, _kwargs| {
                let mut args = args.into_iter();
                let mut table =
                    Eval::move_or_clone_table(globals, args.next().unwrap())?.map_move();
                for arg in args {
                    let arg = Eval::move_or_clone_table(globals, arg)?.map_move();
                    table.extend(arg);
                }
                Ok(Value::Table(Table::new(table).into()))
            },
        ),
        NativeFunction::sdnew(
            "minus",
            (&["self"], &[], Some("keys"), None),
            Some("Returns a new Table with all given keys removed"),
            |globals, args, _kwargs| {
                let mut args = args.into_iter();
                let mut table =
                    Eval::move_or_clone_table(globals, args.next().unwrap())?.map_move();
                for key in args {
                    let key = Eval::expect_symbol(globals, &key)?;
                    table.remove(&key);
                }
                Ok(Value::Table(Table::new(table).into()))
            },
        ),
    ]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
    .collect();
    let static_methods = vec![
        NativeFunction::snew(
            "__call",
            (&[], &[], None, Some("kwargs")),
            |_globals, _args, kwargs| Ok(Value::Table(Table::new(kwargs.unwrap()).into())),
        ),
        NativeFunction::snew(
            "from_iterable",
            (&["iterable"], &[], None, None),
            |globals, args, _kwargs| Eval::table_from_iterable(globals, &args[0]),
        ),
    ]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
    .collect();

    Class::new0(
        ClassKind::NativeClass,
        "Table".into(),
        vec![base],
        None,
        methods,
        static_methods,
    )
    .into()
}
