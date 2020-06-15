use super::Val;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct Scope {
    id_count: Rc<RefCell<usize>>,
    opts: Rc<Options>,
    parent: Option<Rc<RefCell<Scope>>>,
    map: HashMap<Rc<String>, Val>,
}

impl Scope {
    pub fn new_root(opts: Options) -> Rc<RefCell<Scope>> {
        Rc::new(RefCell::new(Scope {
            id_count: Rc::new(RefCell::new(0)),
            opts: Rc::new(opts),
            parent: None,
            map: HashMap::new(),
        }))
    }
    pub fn new(parent: Rc<RefCell<Scope>>) -> Rc<RefCell<Scope>> {
        let id_count = parent.borrow().id_count.clone();
        let opts = parent.borrow().opts.clone();
        Rc::new(RefCell::new(Scope {
            id_count,
            opts,
            parent: Some(parent),
            map: HashMap::new(),
        }))
    }
    pub fn get(&self, key: &Rc<String>) -> Option<Val> {
        match self.map.get(key) {
            Some(val) => Some(val.clone()),
            None => match &self.parent {
                Some(parent) => parent.borrow().get(key),
                None => None,
            },
        }
    }
    pub fn set(&mut self, key: Rc<String>, val: Val) {
        self.map.insert(key, val);
    }
    pub fn new_id(&self) -> usize {
        let mut id_count = self.id_count.borrow_mut();
        let id: usize = *id_count;
        *id_count += 1;
        id
    }
    pub fn opts(&self) -> &Options {
        &self.opts
    }
}

pub struct Options {
    pub print: Box<dyn Fn(&Val) -> Result<Val, String>>,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            print: Box::new(|x| {
                println!("{}", x);
                Ok(Val::Nil)
            }),
        }
    }
}
