use super::*;

#[derive(PartialEq)]
pub struct Object {
    cls: Rc<Class>,
    map: HashMap<RcStr, RefCell<Value>>,
}

impl cmp::PartialOrd for Object {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        (self as *const Self).partial_cmp(&(other as *const Self))
    }
}

impl fmt::Debug for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{} object>", self.cls.name())
    }
}

impl Object {
    pub fn builder(cls: Rc<Class>) -> ObjectBuilder {
        ObjectBuilder {
            cls,
            map: HashMap::new(),
        }
    }
    pub fn new(cls: Rc<Class>, map: HashMap<RcStr, RefCell<Value>>) -> Self {
        Self {
            cls,
            map,
        }
    }
    pub fn cls(&self) -> &Rc<Class> {
        &self.cls
    }
    pub fn map(&self) -> &HashMap<RcStr, RefCell<Value>> {
        &self.map
    }
}

pub struct ObjectBuilder {
    cls: Rc<Class>,
    map: HashMap<RcStr, RefCell<Value>>,
}

impl ObjectBuilder {
    pub fn build(self) -> Object {
        Object::new(self.cls, self.map)
    }
}

impl From<Object> for Value {
    fn from(obj: Object) -> Self {
        Self::Object(Rc::new(obj))
    }
}

impl From<Rc<Object>> for Value {
    fn from(obj: Rc<Object>) -> Self {
        Self::Object(obj)
    }
}

impl From<&Rc<Object>> for Value {
    fn from(obj: &Rc<Object>) -> Self {
        Self::Object(obj.clone())
    }
}
