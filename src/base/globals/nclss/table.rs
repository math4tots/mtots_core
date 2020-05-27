use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::NativeFunction;
use crate::SymbolRegistryHandle;
use crate::Table;
use crate::Value;

use std::rc::Rc;

pub(super) fn mkcls(sr: &SymbolRegistryHandle, base: Rc<Class>) -> Rc<Class> {
    let methods = vec![
        NativeFunction::simple0(sr, "len", &["self"], |globals, args, _kwargs| {
            let s = Eval::expect_table(globals, &args[0])?;
            Ok(Value::Int(s.len() as i64))
        }),
        NativeFunction::snew(
            sr,
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
            sr,
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
    ]
    .into_iter()
    .map(|f| (sr.intern_rcstr(f.name()), Value::from(f)))
    .collect();
    let static_methods = vec![NativeFunction::snew(
        sr,
        "__call",
        (&[], &[], None, Some("kwargs")),
        |_globals, _args, kwargs| Ok(Value::Table(Table::new(kwargs.unwrap()).into())),
    )]
    .into_iter()
    .map(|f| (sr.intern_rcstr(f.name()), Value::from(f)))
    .collect();

    Class::new0(
        ClassKind::NativeClass,
        "Table".into(),
        vec![base],
        methods,
        static_methods,
    )
    .into()
}
