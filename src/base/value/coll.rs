use super::*;

/// Wrapper around RefCell<Vec<Value>>
/// Having a wrapper keeps the possibility open for e.g.
/// caching hash values, or mutability locks
#[derive(PartialEq)]
pub struct List {
    vec: RefCell<Vec<Value>>,
}

impl List {
    pub fn borrow(&self) -> Ref<Vec<Value>> {
        self.vec.borrow()
    }
    pub fn borrow_mut(&self) -> RefMut<Vec<Value>> {
        self.vec.borrow_mut()
    }
    pub fn into_inner(self) -> Vec<Value> {
        self.vec.into_inner()
    }
    pub fn generator(list: Rc<Self>) -> NativeGenerator {
        let mut i = 0;
        NativeGenerator::new("list-iterator", move |_globals, _arg| {
            if let Some(x) = list.borrow().get(i).cloned() {
                i += 1;
                ResumeResult::Yield(x)
            } else {
                ResumeResult::Return(Value::Nil)
            }
        })
    }
}

#[derive(PartialEq, Eq)]
pub struct Set {
    set: RefCell<IndexSet<Key>>,
}

impl Set {
    pub fn borrow(&self) -> Ref<IndexSet<Key>> {
        self.set.borrow()
    }
    pub fn borrow_mut(&self) -> RefMut<IndexSet<Key>> {
        self.set.borrow_mut()
    }
    pub fn into_inner(self) -> IndexSet<Key> {
        self.set.into_inner()
    }
    pub fn sorted_keys(&self) -> Vec<Key> {
        let mut vec: Vec<_> = self.borrow().clone().into_iter().collect();
        vec.sort();
        vec
    }
}

#[derive(PartialEq)]
pub struct Map {
    map: RefCell<IndexMap<Key, Value>>,
}

impl Map {
    pub fn borrow(&self) -> Ref<IndexMap<Key, Value>> {
        self.map.borrow()
    }
    pub fn borrow_mut(&self) -> RefMut<IndexMap<Key, Value>> {
        self.map.borrow_mut()
    }
    pub fn into_inner(self) -> IndexMap<Key, Value> {
        self.map.into_inner()
    }
    pub fn to_string_keys(&self) -> Result<HashMap<RcStr, Value>> {
        let mut ret = HashMap::new();
        for (key, val) in self.borrow().iter() {
            match key {
                Key::String(string) => {
                    ret.insert(string.clone(), val.clone());
                }
                _ => return Err(rterr!("Expected string keys but got {:?}", key)),
            }
        }
        Ok(ret)
    }
}

impl From<Vec<Value>> for Value {
    fn from(vec: Vec<Value>) -> Self {
        Self::List(
            List {
                vec: RefCell::new(vec),
            }
            .into(),
        )
    }
}

impl From<IndexSet<Key>> for Value {
    fn from(set: IndexSet<Key>) -> Self {
        Self::Set(
            Set {
                set: RefCell::new(set),
            }
            .into(),
        )
    }
}
