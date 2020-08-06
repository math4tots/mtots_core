use super::*;

#[allow(non_snake_case)]
pub struct ClassManager {
    pub Nil: Rc<Class>,
    pub Bool: Rc<Class>,
    pub Number: Rc<Class>,
    pub String: Rc<Class>,
    pub List: Rc<Class>,
    pub Set: Rc<Class>,
    pub Map: Rc<Class>,
    pub Function: Rc<Class>,
    pub NativeFunction: Rc<Class>,
    pub Generator: Rc<Class>,
    pub NativeGenerator: Rc<Class>,
    pub Class: Rc<Class>,
    pub Module: Rc<Class>,
}

impl ClassManager {
    #[allow(non_snake_case)]
    pub(super) fn new() -> Self {
        let Nil = Class::new("Nil".into(), HashMap::new(), HashMap::new());
        let Bool = Class::new("Bool".into(), HashMap::new(), HashMap::new());
        let Number = Class::new("Number".into(), HashMap::new(), HashMap::new());
        let String = Class::new("String".into(), HashMap::new(), HashMap::new());
        let List = Class::new("List".into(), HashMap::new(), HashMap::new());
        let Set = Class::new(
            "Set".into(),
            HashMap::new(),
            Class::map_from_funcs(vec![NativeFunction::new(
                "__call",
                ["x"],
                |globals, args| {
                    let x = args.into_iter().next().unwrap();
                    let set = x.unpack_into_set(globals)?;
                    Ok(set.into())
                },
            )]),
        );
        let Map = Class::new("Map".into(), HashMap::new(), HashMap::new());
        let Function = Class::new("Function".into(), HashMap::new(), HashMap::new());
        let NativeFunction = Class::new("NativeFunction".into(), HashMap::new(), HashMap::new());
        let Generator = Class::new("Generator".into(), HashMap::new(), HashMap::new());
        let NativeGenerator = Class::new("NativeGenerator".into(), HashMap::new(), HashMap::new());
        let Class = Class::new("Class".into(), HashMap::new(), HashMap::new());
        let Module = Class::new("Module".into(), HashMap::new(), HashMap::new());
        Self {
            Nil,
            Bool,
            Number,
            String,
            List,
            Set,
            Map,
            Function,
            NativeFunction,
            Generator,
            NativeGenerator,
            Class,
            Module,
        }
    }

    pub fn get_class<'a>(&'a self, value: &'a Value) -> &'a Rc<Class> {
        match value {
            Value::Invalid => panic!("get_class(Invalid)"),
            Value::Nil => &self.Nil,
            Value::Bool(..) => &self.Bool,
            Value::Number(..) => &self.Number,
            Value::String(..) => &self.String,
            Value::List(..) => &self.List,
            Value::Set(..) => &self.Set,
            Value::Map(..) => &self.Map,
            Value::Table(table) => table.cls(),
            Value::Function(..) => &self.Function,
            Value::NativeFunction(..) => &self.NativeFunction,
            Value::Generator(..) => &self.Generator,
            Value::NativeGenerator(..) => &self.NativeGenerator,
            Value::Class(..) => &self.Class,
            Value::Module(..) => &self.Module,
            Value::Handle(handle) => handle.cls(),
        }
    }

    pub fn list(&self) -> Vec<&Rc<Class>> {
        vec![
            &self.Nil,
            &self.Bool,
            &self.Number,
            &self.String,
            &self.List,
            &self.Set,
            &self.Map,
            &self.Function,
            &self.NativeFunction,
            &self.Generator,
            &self.NativeGenerator,
            &self.Class,
            &self.Module,
        ]
    }
}