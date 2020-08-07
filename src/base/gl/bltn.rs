use super::*;

impl Globals {
    pub fn new_builtins(class_manager: &ClassManager) -> HashMap<RcStr, Value> {
        let mut map = HashMap::new();
        for cls in class_manager.list() {
            map.insert(cls.name().clone(), cls.clone().into());
        }

        let funcs = vec![
            NativeFunction::new("print", ["x"], None, |_globals, args, _| {
                let x = args.into_iter().next().unwrap();
                println!("{}", x);
                Ok(Value::Nil)
            }),
            NativeFunction::new("str", ["x"], None, |_globals, args, _| {
                Ok(args.into_iter().next().unwrap().convert_to_rcstr().into())
            }),
            NativeFunction::new("sorted", ["x"], None, |globals, args, _| {
                let mut list = args.into_iter().next().unwrap().unpack(globals)?;
                list.sort_by(|a, b| a.partial_cmp(&b).unwrap_or(cmp::Ordering::Equal));
                Ok(list.into())
            }),
            NativeFunction::new("assert_eq", ["a", "b"], None, |_globals, args, _| {
                let mut args = args.into_iter();
                let a = args.next().unwrap();
                let b = args.next().unwrap();
                if a != b {
                    Err(rterr!("Expected {:?} to equal {:?}", a, b))
                } else {
                    Ok(Value::Nil)
                }
            }),
            NativeFunction::new("getattr", ["owner", "name"], None, |_globals, args, _| {
                let mut args = args.into_iter();
                let owner = args.next().unwrap();
                let name = args.next().unwrap().into_string()?;
                owner.getattr(&name)
            }),
            NativeFunction::new("__import", ["name"], None, |globals, args, _| {
                let name = args.into_iter().next().unwrap();
                let name = name.string()?;
                Ok(globals.load(name)?.into())
            }),
            NativeFunction::new("__module_keys", ["module"], None, |_globals, args, _| {
                let module = args.into_iter().next().unwrap().into_module()?;
                Ok(module
                    .map()
                    .iter()
                    .map(|(key, _)| Value::from(key.clone()))
                    .collect::<Vec<_>>()
                    .into())
            }),
            NativeFunction::new("__disasm", ["func"], None, |_globals, args, _| {
                let func = args.into_iter().next().unwrap();
                let func = func.function()?;
                let string = func.code().disasm()?;
                Ok(Value::from(string))
            }),
            NativeFunction::new("__main", (), None, |globals, _args, _| {
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
