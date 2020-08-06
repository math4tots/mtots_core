use super::*;

pub struct Module {
    name: RcStr,
    map: HashMap<RcStr, Rc<RefCell<Value>>>,
    docmap: Rc<HashMap<RcStr, RcStr>>,
}

impl Module {
    pub fn new(name: RcStr, vars: Vec<RcStr>, docmap: Rc<HashMap<RcStr, RcStr>>) -> Self {
        Self::new_with_cells(
            name,
            vars.into_iter()
                .map(|var| (var, Rc::new(RefCell::new(Value::Invalid))))
                .collect(),
            docmap,
        )
    }
    pub fn new_with_cells(
        name: RcStr,
        vars: Vec<(RcStr, Rc<RefCell<Value>>)>,
        docmap: Rc<HashMap<RcStr, RcStr>>,
    ) -> Self {
        Self {
            name,
            map: vars.into_iter().collect(),
            docmap,
        }
    }
    pub fn name(&self) -> &RcStr {
        &self.name
    }
    pub fn map(&self) -> &HashMap<RcStr, Rc<RefCell<Value>>> {
        &self.map
    }
    pub fn get(&self, name: &RcStr) -> Option<Value> {
        self.map.get(name).map(|cell| cell.borrow().clone())
    }
    pub fn docmap(&self) -> &Rc<HashMap<RcStr, RcStr>> {
        &self.docmap
    }
}

impl cmp::PartialEq for Module {
    fn eq(&self, other: &Self) -> bool {
        self as *const _ == other as *const _
    }
}

impl cmp::PartialOrd for Module {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        (self as *const Self as usize).partial_cmp(&(other as *const Self as usize))
    }
}

impl fmt::Debug for Module {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<module {}>", self.name)
    }
}
