use super::*;

impl Globals {
    pub fn new_builtins(class_manager: &ClassManager) -> HashMap<RcStr, Value> {
        let mut map = HashMap::new();
        for cls in class_manager.list() {
            map.insert(cls.name().clone(), cls.clone().into());
        }

        let funcs = vec![
            NativeFunction::new("print", ["x"], |_globals, args, _| {
                let x = args.into_iter().next().unwrap();
                println!("{}", x);
                Ok(Value::Nil)
            }),
            NativeFunction::new("str", ["x"], |_globals, args, _| {
                Ok(args.into_iter().next().unwrap().into_rcstr().into())
            }),
            NativeFunction::new("sorted", ["x"], |globals, args, _| {
                let mut list = args.into_iter().next().unwrap().unpack(globals)?;
                list.sort_by(|a, b| a.partial_cmp(&b).unwrap_or(cmp::Ordering::Equal));
                Ok(list.into())
            }),
            NativeFunction::new("__import", ["name"], |globals, args, _| {
                let name = args.into_iter().next().unwrap();
                let name = name.string()?;
                Ok(globals.load(name)?.into())
            }),
            NativeFunction::new("__main", (), |globals, _args, _| {
                Ok(globals
                    .get_main()
                    .as_ref()
                    .map(Value::from)
                    .unwrap_or(Value::Nil))
            }),
        ];

        for func in funcs {
            map.insert(func.name().clone(), func.into());
        }

        map
    }
}
