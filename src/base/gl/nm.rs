use super::*;

pub struct NativeModule {
    name: RcStr,
    data: Box<dyn FnOnce(&mut Globals) -> NativeModuleData>,
}

pub struct NativeModuleData {
    pub(super) fields: Vec<RcStr>,
    pub(super) init:
        Box<dyn FnOnce(&mut Globals, &HashMap<RcStr, Rc<RefCell<Value>>>) -> Result<()>>,
    pub(super) docmap: HashMap<RcStr, RcStr>,
}

impl NativeModule {
    pub fn new<N, F>(name: N, f: F) -> Self
    where
        N: Into<RcStr>,
        F: FnOnce(&mut Globals, NativeModuleBuilder) -> NativeModuleData + 'static,
    {
        Self {
            name: name.into(),
            data: Box::new(|globals| {
                f(
                    globals,
                    NativeModuleBuilder {
                        deps: vec![],
                        fields: vec![],
                        docmap: HashMap::new(),
                        action: None,
                    },
                )
            }),
        }
    }
    pub fn name(&self) -> &RcStr {
        &self.name
    }
    pub fn data(self, globals: &mut Globals) -> NativeModuleData {
        (self.data)(globals)
    }
}

pub struct NativeModuleBuilder {
    deps: Vec<RcStr>,
    fields: Vec<(
        RcStr,
        Box<dyn FnOnce(&mut Globals, &HashMap<RcStr, Rc<RefCell<Value>>>) -> Result<Value>>,
    )>,
    docmap: HashMap<RcStr, RcStr>,
    action:
        Option<Box<dyn FnOnce(&mut Globals, &HashMap<RcStr, Rc<RefCell<Value>>>) -> Result<()>>>,
}

impl NativeModuleBuilder {
    pub fn dep<N: Into<RcStr>>(mut self, name: N) -> Self {
        self.deps.push(name.into());
        self
    }
    pub fn field<N, D, F>(mut self, name: N, doc: D, body: F) -> Self
    where
        N: Into<RcStr>,
        D: Into<DocStr>,
        F: FnOnce(&mut Globals, &HashMap<RcStr, Rc<RefCell<Value>>>) -> Result<Value> + 'static,
    {
        let name = name.into();
        if let Some(doc) = doc.into().get() {
            self.docmap.insert(name.clone(), doc.clone());
        }
        self.fields.push((name, Box::new(body)));
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
        if let Some(doc) = doc.get() {
            self.docmap.insert(name.clone(), doc.clone());
        }
        self.fields.push((
            name.clone(),
            Box::new(|_globals, _map| Ok(NativeFunction::new(name, argspec, doc, body).into())),
        ));
        self
    }
    pub fn action<F>(mut self, body: F) -> NativeModuleData
    where
        F: FnOnce(&mut Globals, &HashMap<RcStr, Rc<RefCell<Value>>>) -> Result<()> + 'static,
    {
        self.action = Some(Box::new(body));
        self.build()
    }
    pub fn build(self) -> NativeModuleData {
        let fields = self.fields;
        let docmap = self.docmap;
        let action = self.action;
        let deps = self.deps;
        NativeModuleData {
            fields: fields.iter().map(|(name, _)| name.clone()).collect(),
            init: Box::new(move |globals, map| -> Result<()> {
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
            }),
            docmap,
        }
    }
}
