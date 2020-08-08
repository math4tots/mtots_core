use super::*;

pub(super) fn new() -> Rc<Class> {
    Class::new(
        "Nil".into(),
        Class::map_from_funcs(vec![
            NativeFunction::new("map", ["self", "x", "f"], "", |globals, args, _| {
                let mut args = args.into_iter();
                args.next().unwrap(); // nil
                let x = args.next().unwrap();
                let f = args.next().unwrap();
                if x.is_nil() {
                    Ok(Value::Nil)
                } else {
                    f.apply(globals, vec![x], None)
                }
            }),
            NativeFunction::new("vmap", ["self", "x", "new"], "", |_globals, args, _| {
                let mut args = args.into_iter();
                args.next().unwrap(); // nil
                let x = args.next().unwrap();
                let new = args.next().unwrap();
                if x.is_nil() {
                    Ok(Value::Nil)
                } else {
                    Ok(new)
                }
            }),
            NativeFunction::new("get", ["self", "x", "default"], "", |_globals, args, _| {
                let mut args = args.into_iter();
                args.next().unwrap(); // nil
                let x = args.next().unwrap();
                let default = args.next().unwrap();
                if x.is_nil() {
                    Ok(default)
                } else {
                    Ok(x)
                }
            }),
            NativeFunction::new("fget", ["self", "x", "f"], "", |globals, args, _| {
                let mut args = args.into_iter();
                args.next().unwrap(); // nil
                let x = args.next().unwrap();
                let f = args.next().unwrap();
                if x.is_nil() {
                    f.apply(globals, vec![], None)
                } else {
                    Ok(x)
                }
            }),
        ]),
        HashMap::new(),
    )
}
