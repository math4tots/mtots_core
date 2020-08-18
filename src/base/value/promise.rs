use crate::ordie;
use crate::Globals;
use crate::Result;
use crate::Value;
use std::cell::RefCell;
use std::cmp;
use std::fmt;
use std::rc::Rc;

/// Javascript-style promise model.
/// Because the Rust one is a lot more involved, and doesn't make it easy
/// to pass around a context object like Globals.
///
/// We also make use of the fact that mtots will never be
/// multi-threaded (like JS), so we don't worry about Send.
///
pub enum Promise {
    Pending(Vec<Box<dyn FnOnce(&mut Globals, Result<Value>)>>),
    Resolved(Result<Value>),
}

impl Promise {
    /// Construct a new promise with the given result
    pub fn unit(result: Result<Value>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Promise::Resolved(result)))
    }

    /// Construct a new promise with a callback that will ensure that
    /// a resolve callback will be called
    pub fn new<F>(globals: &mut Globals, f: F) -> Rc<RefCell<Self>>
    where
        F: FnOnce(&mut Globals, Box<dyn FnOnce(&mut Globals, Result<Value>)>),
    {
        let ret = Rc::new(RefCell::new(Self::Pending(vec![])));
        let rc = ret.clone();
        f(
            globals,
            Box::new(move |globals, result| {
                let mut ref_ = rc.borrow_mut();
                let ref_: &mut Promise = &mut ref_;
                {
                    let vec = match ref_ {
                        Self::Pending(vec) => vec,
                        Self::Resolved(_) => panic!("Promise resolved more than once"),
                    };
                    for callback in vec.drain(..) {
                        callback(globals, result.clone());
                    }
                }
                *ref_ = Self::Resolved(result);
            }),
        );
        ret
    }

    /// Register a callback to be called for when this promise is resolved.
    /// If the promise is resolved, the callback is called immediately.
    pub fn register<F>(&mut self, globals: &mut Globals, f: F)
    where
        F: FnOnce(&mut Globals, Result<Value>) + 'static,
    {
        match self {
            Self::Pending(vec) => vec.push(Box::new(f)),
            Self::Resolved(result) => f(globals, result.clone()),
        }
    }

    /// Ensures that if this promise errors out, that it will
    /// panic and dump an error message for the user to see
    pub fn ordie(&mut self, globals: &mut Globals) {
        self.register(globals, |globals, result| {
            ordie(globals, result);
        });
    }

    pub fn map<F>(&mut self, globals: &mut Globals, f: F) -> Rc<RefCell<Self>>
    where
        F: FnOnce(&mut Globals, Result<Value>) -> Result<Value> + 'static,
    {
        Self::new(globals, |globals, resolve| {
            self.register(globals, |globals, result| {
                let result = f(globals, result);
                resolve(globals, result);
            });
        })
    }

    pub fn flat_map<F>(&mut self, globals: &mut Globals, f: F) -> Rc<RefCell<Self>>
    where
        F: FnOnce(&mut Globals, Result<Value>) -> Rc<RefCell<Self>> + 'static,
    {
        Self::new(globals, |globals, resolve| {
            self.register(globals, |globals, result| {
                f(globals, result)
                    .borrow_mut()
                    .register(globals, |globals, result| {
                        resolve(globals, result);
                    });
            });
        })
    }
}

impl cmp::PartialEq for Promise {
    fn eq(&self, other: &Self) -> bool {
        self as *const _ == other as *const _
    }
}

impl fmt::Debug for Promise {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<Promise@{:x?}>", self as *const _)
    }
}
