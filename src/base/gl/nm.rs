use super::*;

pub struct NativeModule {
    name: RcStr,
    data: Box<dyn FnOnce(&mut Globals) -> NativeModuleData>,
}

pub struct NativeModuleData {
    pub(super) fields: Vec<RcStr>,
    pub(super) init:
        Box<dyn FnOnce(&mut Globals, &HashMap<RcStr, Rc<RefCell<Value>>>) -> Result<()>>,
    pub(super) doc: Option<RcStr>,
    pub(super) docmap: HashMap<RcStr, RcStr>,
}

impl NativeModule {
    pub fn new<N, F>(name: N, f: F) -> Self
    where
        N: Into<RcStr>,
        F: FnOnce(&mut NativeModuleBuilder) + 'static,
    {
        let name = name.into();
        Self {
            name: name.clone(),
            data: Box::new(|_globals| {
                let mut builder = NativeModuleBuilder {
                    name,
                    doc: None,
                    deps: vec![],
                    fields: vec![],
                    docmap: HashMap::new(),
                    action: None,
                };
                f(&mut builder);
                builder.build()
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
    name: RcStr,
    doc: Option<RcStr>,
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
    pub fn doc<D: Into<RcStr>>(&mut self, doc: D) -> &mut Self {
        self.doc = Some(doc.into());
        self
    }
    pub fn dep<N: Into<RcStr>>(&mut self, name: N) -> &mut Self {
        self.deps.push(name.into());
        self
    }
    pub fn field<N, D, F>(&mut self, name: N, doc: D, body: F) -> &mut Self
    where
        N: Into<RcStr>,
        D: Into<DocStr>,
        F: FnOnce(&mut Globals, &HashMap<RcStr, Rc<RefCell<Value>>>) -> Result<Value> + 'static,
    {
        let name = name.into();
        if let Some(doc) = doc.into().as_ref() {
            self.docmap.insert(name.clone(), doc.clone());
        }
        self.fields.push((name, Box::new(body)));
        self
    }
    pub fn val<N, D, V>(&mut self, name: N, doc: D, value: V) -> &mut Self
    where
        N: Into<RcStr>,
        D: Into<DocStr>,
        V: Into<Value>,
    {
        let value = value.into();
        self.field(name, doc, |_, _| Ok(value))
    }
    pub fn func<N, A, D, B>(&mut self, name: N, argspec: A, doc: D, body: B) -> &mut Self
    where
        N: Into<RcStr>,
        A: Into<ArgSpec>,
        D: Into<DocStr>,
        B: Fn(&mut Globals, Vec<Value>, Option<HashMap<RcStr, Value>>) -> Result<Value> + 'static,
    {
        let name = name.into();
        let argspec = argspec.into();
        let doc = doc.into();
        self.field(name.clone(), doc.as_ref().clone(), |_globals, _map| {
            Ok(NativeFunction::new(name, argspec, doc, body).into())
        })
    }
    pub fn class<T, F>(&mut self, name: &str, f: F) -> &mut Self
    where
        T: Any,
        F: FnOnce(&mut NativeClassBuilder<T>),
    {
        let mut builder = NativeClassBuilder {
            module_builder: self,
            typeid: TypeId::of::<T>(),
            typename: std::any::type_name::<T>(),
            name: name.into(),
            doc: None,
            map: HashMap::new(),
            static_map: HashMap::new(),
            behavior: Behavior::builder_for_handle(),
        };
        f(&mut builder);
        builder.build()
    }
    pub fn action<F>(&mut self, body: F)
    where
        F: FnOnce(&mut Globals, &HashMap<RcStr, Rc<RefCell<Value>>>) -> Result<()> + 'static,
    {
        if self.action.is_some() {
            panic!(
                "An action is already registered for this builder ({:?})",
                self.name
            );
        }
        self.action = Some(Box::new(body));
    }
    fn build(self) -> NativeModuleData {
        {
            let mut set = HashSet::new();
            for (field, _) in &self.fields {
                if set.contains(&field) {
                    panic!(
                        "Duplicate definition of field {:?} in native module {:?}",
                        field, self.name,
                    );
                }
                set.insert(field);
            }
        }
        let fields = self.fields;
        let doc = self.doc;
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
            doc,
            docmap,
        }
    }
}

pub struct NativeClassBuilder<'a, T: Any> {
    module_builder: &'a mut NativeModuleBuilder,
    typeid: TypeId,
    typename: &'static str,
    name: RcStr,
    doc: Option<RcStr>,
    map: HashMap<RcStr, Value>,
    static_map: HashMap<RcStr, Value>,
    behavior: HandleBehaviorBuilder<T>,
}

impl<'a, T: Any> NativeClassBuilder<'a, T> {
    pub fn doc<D: Into<DocStr>>(&mut self, doc: D) -> &mut Self {
        self.doc = doc.into().get();
        self
    }
    /// Declare an instance method
    pub fn ifunc<N, A, D, B>(&mut self, name: N, argspec: A, doc: D, body: B) -> &mut Self
    where
        N: Into<RcStr>,
        A: Into<ArgSpec>,
        D: Into<DocStr>,
        B: Fn(Handle<T>, &mut Globals, Vec<Value>, Option<HashMap<RcStr, Value>>) -> Result<Value>
            + 'static,
    {
        let name = name.into();
        let mut argspec = argspec.into();
        argspec.add_self_parameter();
        let doc = doc.into();
        let func = NativeFunction::new(
            name.clone(),
            argspec,
            doc,
            move |globals, mut args, kwargs| {
                let handle = args.remove(0).into_handle::<T>()?;
                body(handle, globals, args, kwargs)
            },
        );
        self.map.insert(name, func.into());
        self
    }
    /// Declare a static method
    pub fn sfunc<N, A, D, B>(&mut self, name: N, argspec: A, doc: D, body: B) -> &mut Self
    where
        N: Into<RcStr>,
        A: Into<ArgSpec>,
        D: Into<DocStr>,
        B: Fn(&mut Globals, Vec<Value>, Option<HashMap<RcStr, Value>>) -> Result<Value> + 'static,
    {
        let name = name.into();
        let argspec = argspec.into();
        let doc = doc.into();
        let func = NativeFunction::new(name.clone(), argspec, doc, body);
        self.static_map.insert(name, func.into());
        self
    }
    /// Customize the default behavior when 'str' function is called
    pub fn str<F>(&mut self, f: F) -> &mut Self
    where
        F: Fn(&T) -> RcStr + 'static,
    {
        self.behavior.str(f);
        self
    }
    /// Customize the default behavior when 'repr' function is called
    pub fn repr<F>(&mut self, f: F) -> &mut Self
    where
        F: Fn(&T) -> RcStr + 'static,
    {
        self.behavior.repr(f);
        self
    }
    fn build(self) -> &'a mut NativeModuleBuilder {
        let mb = self.module_builder;
        let typeid = self.typeid;
        let typename = self.typename;
        let name = self.name;
        let map = self.map;
        let static_map = self.static_map;
        let behavior = self.behavior;
        mb.field(name.clone(), self.doc, move |globals, _| {
            let cls = Class::new_with_behavior(name, map, static_map, Some(behavior.build()));
            globals.set_handle_class_by_id(typeid, typename, cls.clone())?;
            Ok(cls.into())
        })
    }
}
