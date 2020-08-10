use super::*;

pub struct Class {
    name: RcStr,
    map: HashMap<RcStr, Value>,
    static_map: HashMap<RcStr, Value>,
    behavior: Option<Behavior>,
}

impl Class {
    pub fn new(
        name: RcStr,
        map: HashMap<RcStr, Value>,
        static_map: HashMap<RcStr, Value>,
    ) -> Rc<Self> {
        Self::new_with_behavior(name, map, static_map, None)
    }
    pub fn new_with_behavior(
        name: RcStr,
        map: HashMap<RcStr, Value>,
        static_map: HashMap<RcStr, Value>,
        behavior: Option<Behavior>,
    ) -> Rc<Self> {
        Rc::new(Self {
            name,
            map,
            static_map,
            behavior,
        })
    }
    pub fn name(&self) -> &RcStr {
        &self.name
    }
    pub fn map(&self) -> &HashMap<RcStr, Value> {
        &self.map
    }
    pub fn static_map(&self) -> &HashMap<RcStr, Value> {
        &self.static_map
    }
    pub fn get_call(&self) -> Option<Value> {
        self.static_map.get("__call").cloned()
    }
    pub fn behavior(&self) -> &Behavior {
        match &self.behavior {
            Some(b) => b,
            _ => panic!(
                "Behavior requested on class with no configured behavior ({:?})",
                self.name
            ),
        }
    }

    /// Convenience method for creating a map or static_map
    /// from some native functions
    pub fn map_from_funcs(funcs: Vec<NativeFunction>) -> HashMap<RcStr, Value> {
        let mut map = HashMap::new();
        for func in funcs {
            map.insert(func.name().clone(), func.into());
        }
        map
    }

    pub fn join_class_maps<C: AsRef<Class>>(
        mut map: HashMap<RcStr, Value>,
        classes: Vec<C>,
    ) -> HashMap<RcStr, Value> {
        for cls in classes {
            let cls = cls.as_ref();
            for (key, val) in &cls.map {
                match map.entry(key.clone()) {
                    Entry::Occupied(_) => {}
                    Entry::Vacant(entry) => {
                        entry.insert(val.clone());
                    }
                }
            }
        }
        map
    }
}

impl cmp::PartialEq for Class {
    fn eq(&self, other: &Self) -> bool {
        self as *const _ == other as *const _
    }
}

impl cmp::PartialOrd for Class {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        (self as *const Self as usize).partial_cmp(&(other as *const Self as usize))
    }
}

impl fmt::Debug for Class {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<class {}>", self.name)
    }
}

/// Special configurable behavior for Table or Handle values
/// These behaviors are special and not set with methods because
/// they need to be run in contexts where
///     1. Globals is not available, and/or
///     2. they are not failable, and so failure would
///         mean panic, making debugging purely from script-land
///         fairly unpleasant
#[derive(Default)]
pub struct Behavior {
    str: Option<Rc<dyn Fn(Value) -> RcStr>>,
    repr: Option<Rc<dyn Fn(Value) -> RcStr>>,
}

impl Behavior {
    pub fn builder_for_handle<T: Any>() -> HandleBehaviorBuilder<T> {
        HandleBehaviorBuilder::<T> {
            behavior: Default::default(),
            phantom: PhantomData,
        }
    }
    pub fn str(&self) -> &Option<Rc<dyn Fn(Value) -> RcStr>> {
        &self.str
    }
    pub fn repr(&self) -> &Option<Rc<dyn Fn(Value) -> RcStr>> {
        &self.repr
    }
}

#[derive(Default)]
pub struct HandleBehaviorBuilder<T: Any> {
    behavior: Behavior,
    phantom: PhantomData<fn(T) -> T>,
}

impl<T: Any> HandleBehaviorBuilder<T> {
    pub fn build(self) -> Behavior {
        self.behavior
    }
    pub fn str<F>(&mut self, f: F) -> &mut Self
    where
        F: Fn(&T) -> RcStr + 'static,
    {
        self.behavior.str = Some(Rc::new(move |value| {
            let handle = value.into_handle::<T>().unwrap();
            let string = f(&handle.borrow());
            string
        }));
        self
    }
    pub fn repr<F>(&mut self, f: F) -> &mut Self
    where
        F: Fn(&T) -> RcStr + 'static,
    {
        self.behavior.repr = Some(Rc::new(move |value| {
            let handle = value.into_handle::<T>().unwrap();
            let string = f(&handle.borrow());
            string
        }));
        self
    }
}
