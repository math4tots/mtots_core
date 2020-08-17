use super::*;

/// The priamry user defined object instances
/// Essentially a class + bag of fields
#[derive(PartialEq)]
pub struct Table {
    cls: Rc<Class>,
    map: HashMap<RcStr, RefCell<Value>>,
}

impl cmp::PartialOrd for Table {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        (self as *const Self).partial_cmp(&(other as *const Self))
    }
}

impl fmt::Debug for Table {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{} table>", self.cls.name())
    }
}

impl Table {
    pub fn new(cls: Rc<Class>, map: HashMap<RcStr, RefCell<Value>>) -> Self {
        Table { cls, map }
    }
    pub fn cls(&self) -> &Rc<Class> {
        &self.cls
    }
    pub fn map(&self) -> &HashMap<RcStr, RefCell<Value>> {
        &self.map
    }
    pub fn set(&self, key: &RcStr, value: Value) -> Result<()> {
        if self.set_opt(key, value).is_ok() {
            Ok(())
        } else {
            Err(rterr!("Attribute {:?} not found in {:?}", key, self))
        }
    }
    pub fn set_opt(&self, key: &RcStr, value: Value) -> std::result::Result<(), Value> {
        match self.map.get(key) {
            Some(cell) => {
                cell.replace(value);
                Ok(())
            }
            None => Err(value),
        }
    }
}

impl From<Table> for Value {
    fn from(obj: Table) -> Self {
        Self::Table(Rc::new(obj))
    }
}

impl From<Rc<Table>> for Value {
    fn from(obj: Rc<Table>) -> Self {
        Self::Table(obj)
    }
}

impl From<&Rc<Table>> for Value {
    fn from(obj: &Rc<Table>) -> Self {
        Self::Table(obj.clone())
    }
}
