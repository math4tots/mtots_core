use super::*;

impl Globals {
    pub fn new_builtins(class_manager: &ClassManager) -> HashMap<RcStr, Value> {
        let mut map = HashMap::new();
        for cls in class_manager.list() {
            map.insert(cls.name().clone(), cls.clone().into());
        }

        let funcs = vec![
            Builtin::new("print", ["x"], |_globals, args| {
                let x = args.into_iter().next().unwrap();
                println!("{}", x);
                Ok(Value::Nil)
            }),
            Builtin::new("str", ["x"], |_globals, args| {
                Ok(args.into_iter().next().unwrap().into_rcstr().into())
            }),
        ];

        for func in funcs {
            map.insert(func.name().clone(), func.into());
        }

        map
    }
}
