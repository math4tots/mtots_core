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
    pub fn builder(cls: Rc<Class>) -> TableBuilder {
        TableBuilder {
            cls,
            map: HashMap::new(),
        }
    }
    pub fn new(cls: Rc<Class>, map: HashMap<RcStr, RefCell<Value>>) -> Self {
        Table { cls, map }
    }
    pub fn cls(&self) -> &Rc<Class> {
        &self.cls
    }
    pub fn map(&self) -> &HashMap<RcStr, RefCell<Value>> {
        &self.map
    }
}

pub struct TableBuilder {
    cls: Rc<Class>,
    map: HashMap<RcStr, RefCell<Value>>,
}

impl TableBuilder {
    pub fn build(self) -> Table {
        Table::new(self.cls, self.map)
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
