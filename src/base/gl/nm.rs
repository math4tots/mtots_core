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
            deps: vec![],
            fields: vec![],
            action: None,
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
    deps: Vec<RcStr>,
    fields: Vec<(
        RcStr,
        Box<dyn FnOnce(&mut Globals, &HashMap<RcStr, Rc<RefCell<Value>>>) -> Result<Value>>,
    )>,
    action:
        Option<Box<dyn FnOnce(&mut Globals, &HashMap<RcStr, Rc<RefCell<Value>>>) -> Result<()>>>,
}

impl NativeModuleBuilder {
    pub fn dep<N: Into<RcStr>>(mut self, name: N) -> Self {
        self.deps.push(name.into());
        self
    }
    pub fn field<N, F>(mut self, name: N, body: F) -> Self
    where
        N: Into<RcStr>,
        F: FnOnce(&mut Globals, &HashMap<RcStr, Rc<RefCell<Value>>>) -> Result<Value> + 'static,
    {
        self.fields.push((name.into(), Box::new(body)));
        self
    }
    pub fn func<N, A, D, B>(mut self, name: N, argspec: A, doc: D, body: B) -> Self
    where
        N: Into<RcStr>,
        A: Into<ArgSpec>,
        D: Into<DocStr>,
        B: Fn(&mut Globals, Vec<Value>, Option<HashMap<RcStr, Value>>) -> Result<Value> + 'static,
    {
        let name = name.into();
        let argspec = argspec.into();
        let doc = doc.into();
        self.fields.push((
            name.clone(),
            Box::new(|_globals, _map| Ok(NativeFunction::new(name, argspec, doc, body).into())),
        ));
        self
    }
    pub fn action<F>(mut self, body: F) -> NativeModule
    where
        F: FnOnce(&mut Globals, &HashMap<RcStr, Rc<RefCell<Value>>>) -> Result<()> + 'static,
    {
        self.action = Some(Box::new(body));
        self.build()
    }
    pub fn build(self) -> NativeModule {
        let fields = self.fields;
        let action = self.action;
        let deps = self.deps;
        NativeModule {
            name: self.name,
            fields: fields.iter().map(|(name, _)| name.clone()).collect(),
            init: Some(Box::new(move |globals, map| -> Result<()> {
                for dep in deps {
                    globals.load(&dep)?;
                }
                for (name, f) in fields {
                    let value = f(globals, map)?;
                    *map.get(&name).unwrap().borrow_mut() = value;
                }
                if let Some(action) = action {
                    action(globals, map)?;
                }
                Ok(())
            })),
        }
    }
}
