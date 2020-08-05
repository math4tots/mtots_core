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
}

impl cmp::PartialEq for Class {
    fn eq(&self, other: &Self) -> bool {
        self as *const _ == other as *const _
    }
}

impl fmt::Debug for Class {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<class {}>", self.name)
    }
}
