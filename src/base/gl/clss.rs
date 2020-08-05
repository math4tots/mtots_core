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
    pub Builtin: Rc<Class>,
    pub Generator: Rc<Class>,
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
        let Set = Class::new("Set".into(), HashMap::new(), HashMap::new());
        let Map = Class::new("Map".into(), HashMap::new(), HashMap::new());
        let Function = Class::new("Function".into(), HashMap::new(), HashMap::new());
        let Builtin = Class::new("Builtin".into(), HashMap::new(), HashMap::new());
        let Generator = Class::new("Generator".into(), HashMap::new(), HashMap::new());
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
            Builtin,
            Generator,
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
            Value::Function(..) => &self.Function,
            Value::Builtin(..) => &self.Builtin,
            Value::Generator(..) => &self.Generator,
            Value::Class(..) => &self.Class,
            Value::Module(..) => &self.Module,
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
            &self.Builtin,
            &self.Generator,
            &self.Class,
            &self.Module,
        ]
    }
}
