use super::*;

pub struct NativeModule {
    name: RcStr,
    fields: Vec<RcStr>,
    init: Option<Box<dyn FnOnce(&mut Globals, &HashMap<RcStr, Rc<RefCell<Value>>>) -> Result<()>>>,
}

impl NativeModule {
    pub fn builder<N: Into<RcStr>>(name: N) -> NativeModuleBuilder {
        NativeModuleBuilder {
            name: name.into(),
            fields: vec![],
            actions: vec![],
        }
    }
    pub fn name(&self) -> &RcStr {
        &self.name
    }
    pub fn fields(&self) -> &Vec<RcStr> {
        &self.fields
    }
    pub fn init(
        &mut self,
    ) -> Option<Box<dyn FnOnce(&mut Globals, &HashMap<RcStr, Rc<RefCell<Value>>>) -> Result<()>>>
    {
        std::mem::replace(&mut self.init, None)
    }
}

pub struct NativeModuleBuilder {
    name: RcStr,
    fields: Vec<(RcStr, Box<dyn FnOnce(&mut Globals) -> Result<Value>>)>,
    actions: Vec<Box<dyn FnOnce(&mut Globals, &HashMap<RcStr, Rc<RefCell<Value>>>) -> Result<()>>>,
}

impl NativeModuleBuilder {
    pub fn field<N, F>(mut self, name: N, body: F) -> Self
    where
        N: Into<RcStr>,
        F: FnOnce(&mut Globals) -> Result<Value> + 'static,
    {
        self.fields.push((name.into(), Box::new(body)));
        self
    }
    pub fn action<F>(mut self, body: F) -> Self
    where
        F: FnOnce(&mut Globals, &HashMap<RcStr, Rc<RefCell<Value>>>) -> Result<()> + 'static,
    {
        self.actions.push(Box::new(body));
        self
    }
    pub fn build(self) -> NativeModule {
        let fields = self.fields;
        let actions = self.actions;
        NativeModule {
            name: self.name,
            fields: fields.iter().map(|(name, _)| name.clone()).collect(),
            init: Some(Box::new(move |globals, map| -> Result<()> {
                for (name, f) in fields {
                    let value = f(globals)?;
                    *map.get(&name).unwrap().borrow_mut() = value;
                }
                for action in actions {
                    action(globals, map)?;
                }
                Ok(())
            })),
        }
    }
}
