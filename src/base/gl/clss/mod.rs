use super::*;
mod iter;
mod iterble;
mod list;
mod map;
mod nil;
mod num;
mod promise;
mod set;
mod strcls;

#[allow(non_snake_case)]
pub struct ClassManager {
    pub Nil: Rc<Class>,
    pub Bool: Rc<Class>,
    pub Number: Rc<Class>,
    pub Iterable: Rc<Class>,
    pub String: Rc<Class>,
    pub List: Rc<Class>,
    pub Set: Rc<Class>,
    pub Map: Rc<Class>,
    pub Function: Rc<Class>,
    pub NativeFunction: Rc<Class>,
    pub Iterator: Rc<Class>,
    pub Generator: Rc<Class>,
    pub NativeGenerator: Rc<Class>,
    pub Promise: Rc<Class>,
    pub Class: Rc<Class>,
    pub Module: Rc<Class>,
}

impl ClassManager {
    #[allow(non_snake_case)]
    pub(super) fn new() -> Self {
        let Nil = nil::new();
        let Bool = Class::new("Bool".into(), HashMap::new(), HashMap::new());
        let Number = num::new();
        let Iterable = iterble::new();
        let String = strcls::new();
        let List = list::new(&Iterable);
        let Set = set::new(&Iterable);
        let Map = map::new(&Iterable);
        let Function = Class::new(
            "Function".into(),
            Class::map_from_funcs(vec![
                NativeFunction::new("params", ["self"], None, |_globals, args, _| {
                    let f = args.into_iter().next().unwrap().into_function()?;
                    Ok(f.argspec().to_value())
                }),
                NativeFunction::new("doc", ["self"], None, |_globals, args, _| {
                    let f = args.into_iter().next().unwrap().into_function()?;
                    Ok(f.doc().as_ref().map(Value::from).unwrap_or(Value::Nil))
                }),
            ]),
            HashMap::new(),
        );
        let NativeFunction = Class::new(
            "NativeFunction".into(),
            Class::map_from_funcs(vec![
                NativeFunction::new("params", ["self"], None, |_globals, args, _| {
                    let f = args.into_iter().next().unwrap().into_native_function()?;
                    Ok(f.argspec().to_value())
                }),
                NativeFunction::new("doc", ["self"], None, |_globals, args, _| {
                    let f = args.into_iter().next().unwrap().into_native_function()?;
                    Ok(f.doc().as_ref().map(Value::from).unwrap_or(Value::Nil))
                }),
            ]),
            HashMap::new(),
        );
        let Iterator = iter::new(&Iterable);
        let Generator = Class::new(
            "Generator".into(),
            Class::join_class_maps(HashMap::new(), vec![&Iterator]),
            HashMap::new(),
        );
        let NativeGenerator = Class::new(
            "NativeGenerator".into(),
            Class::join_class_maps(HashMap::new(), vec![&Iterator]),
            HashMap::new(),
        );
        let Promise = promise::new();
        let Class = Class::new("Class".into(), HashMap::new(), HashMap::new());
        let Module = Class::new("Module".into(), HashMap::new(), HashMap::new());
        Self {
            Nil,
            Bool,
            Number,
            Iterable,
            String,
            List,
            Set,
            Map,
            Function,
            NativeFunction,
            Iterator,
            Generator,
            NativeGenerator,
            Promise,
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
            Value::Promise(..) => &self.Promise,
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
            &self.Iterable,
            &self.String,
            &self.List,
            &self.Set,
            &self.Map,
            &self.Function,
            &self.NativeFunction,
            &self.Iterator,
            &self.Generator,
            &self.NativeGenerator,
            &self.Promise,
            &self.Class,
            &self.Module,
        ]
    }
}
