use super::*;

pub struct Class {
    name: RcStr,
    map: HashMap<RcStr, Value>,
    static_map: HashMap<RcStr, Value>,
}

impl Class {
    pub fn new(
        name: RcStr,
        map: HashMap<RcStr, Value>,
        static_map: HashMap<RcStr, Value>,
    ) -> Rc<Self> {
        Rc::new(Self {
            name,
            map,
            static_map,
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
