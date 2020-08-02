use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::NativeFunction;
use crate::Value;
use crate::Symbol;
use std::cell::RefCell;
use std::rc::Rc;

pub(super) fn mkcls(base: Rc<Class>) -> Rc<Class> {
    let methods = vec![
        NativeFunction::simple0("get", &["self"], |globals, args, _kwargs| {
            // Gets the value currently stored in this cell
            let cell = Eval::expect_cell(globals, &args[0])?;
            Ok(cell.borrow().clone())
        }),
        NativeFunction::simple0("set", &["self", "x"], |globals, args, _kwargs| {
            // Sets a new value to the cell
            let cell = Eval::expect_cell(globals, &args[0])?;
            *cell.borrow_mut() = args[1].clone();
            Ok(Value::Nil)
        }),
    ]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
    .collect();

    let static_methods = vec![NativeFunction::simple0(
        "__call",
        &["x"],
        |_globals, args, _kwargs| {
            // Creates a new cell initialized with the given value
            Ok(Value::Cell(Rc::new(RefCell::new(args[0].clone()))))
        },
    )]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
    .collect();

    Class::new0(
        ClassKind::NativeClass,
        "Cell".into(),
        vec![base],
        None,
        methods,
        static_methods,
    )
    .into()
}
