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
}

#[derive(PartialEq, Eq)]
pub struct Set {
    set: RefCell<HashSet<Key>>,
}

impl Set {
    pub fn borrow(&self) -> Ref<HashSet<Key>> {
        self.set.borrow()
    }
    pub fn borrow_mut(&self) -> RefMut<HashSet<Key>> {
        self.set.borrow_mut()
    }
    pub fn into_inner(self) -> HashSet<Key> {
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
    map: RefCell<HashMap<Key, Value>>,
}

impl Map {
    pub fn borrow(&self) -> Ref<HashMap<Key, Value>> {
        self.map.borrow()
    }
    pub fn borrow_mut(&self) -> RefMut<HashMap<Key, Value>> {
        self.map.borrow_mut()
    }
    pub fn into_inner(self) -> HashMap<Key, Value> {
        self.map.into_inner()
    }
    pub fn sorted_pairs(&self) -> Vec<(Key, Value)> {
        let mut vec: Vec<_> = self.borrow().clone().into_iter().collect();
        vec.sort_by(|a, b| a.0.cmp(&b.0));
        vec
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

impl From<HashSet<Key>> for Value {
    fn from(set: HashSet<Key>) -> Self {
        Self::Set(
            Set {
                set: RefCell::new(set),
            }
            .into(),
        )
    }
}
