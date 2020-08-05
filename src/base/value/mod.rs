mod cls;
mod coll;
mod conv;
mod cv;
mod format;
mod func;
mod gen;
mod key;
mod m;
use crate::Code;
use crate::Error;
use crate::Frame;
use crate::Globals;
use crate::IndexMap;
use crate::IndexSet;
use crate::RcStr;
use crate::Result;
use std::cell::Ref;
use std::cell::RefCell;
use std::cell::RefMut;
use std::cmp;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt;
use std::rc::Rc;

pub use cls::*;
pub use coll::*;
pub use conv::*;
pub use cv::*;
pub use func::*;
pub use gen::*;
pub use key::*;
pub use m::*;

#[derive(Clone, PartialEq)]
pub enum Value {
    Invalid,
    Nil,
    Bool(bool),
    Number(f64),
    String(RcStr),
    List(Rc<List>),
    Set(Rc<Set>),
    Map(Rc<Map>),
    Function(Rc<Function>),
    Builtin(Rc<Builtin>),
    Generator(Rc<RefCell<Generator>>),
    Class(Rc<Class>),
    Module(Rc<Module>),
}

impl Value {
    pub fn truthy(&self) -> bool {
        match self {
            Self::Invalid => panic!("Value::Invalid::truthy"),
            Self::Nil => false,
            Self::Bool(b) => *b,
            Self::Number(x) => *x != 0.0,
            Self::String(s) => s.len() > 0,
            Self::List(list) => list.borrow().len() > 0,
            Self::Set(set) => set.borrow().len() > 0,
            Self::Map(map) => map.borrow().len() > 0,
            Self::Function(_)
            | Self::Builtin(_)
            | Self::Generator(_)
            | Self::Class(_)
            | Self::Module(_) => true,
        }
    }
    pub fn bool(&self) -> Result<bool> {
        if let Self::Bool(x) = self {
            Ok(*x)
        } else {
            Err(rterr!("Expected bool"))
        }
    }
    pub fn number(&self) -> Result<f64> {
        if let Self::Number(x) = self {
            Ok(*x)
        } else {
            Err(rterr!("Expected number"))
        }
    }
    pub fn string(&self) -> Result<&RcStr> {
        if let Self::String(x) = self {
            Ok(x)
        } else {
            Err(rterr!("Expected string"))
        }
    }
    pub fn list(&self) -> Result<&Rc<List>> {
        if let Self::List(x) = self {
            Ok(x)
        } else {
            Err(rterr!("Expected list"))
        }
    }
    pub fn function(&self) -> Result<&Rc<Function>> {
        if let Self::Function(func) = self {
            Ok(func)
        } else {
            Err(rterr!("Expected function"))
        }
    }
    pub fn class(&self) -> Result<&Rc<Class>> {
        if let Self::Class(cls) = self {
            Ok(cls)
        } else {
            Err(rterr!("Expected class"))
        }
    }
    pub fn module(&self) -> Result<&Rc<Module>> {
        if let Self::Module(m) = self {
            Ok(m)
        } else {
            Err(rterr!("Expected module"))
        }
    }
    pub fn into_rcstr(self) -> RcStr {
        match self {
            Self::String(r) => r,
            _ => format!("{}", self).into(),
        }
    }
    pub fn unwrap_string_or_clone(self) -> Result<String> {
        if let Self::String(r) = self {
            Ok(r.unwrap_or_clone())
        } else {
            Err(rterr!("Expected string"))
        }
    }
    pub fn get_class<'a>(&'a self, globals: &'a Globals) -> &'a Rc<Class> {
        globals.class_manager().get_class(self)
    }
    pub fn getattr_opt(&self, attr: &RcStr) -> Option<Value> {
        match self {
            Self::Class(cls) => cls.static_map().get(attr).map(|cell| cell.borrow().clone()),
            Self::Module(module) => module.get(attr),
            _ => None,
        }
    }
    pub fn getattr(&self, attr: &RcStr) -> Result<Value> {
        match self.getattr_opt(attr) {
            Some(value) => Ok(value),
            None => Err(rterr!("Attribute {:?} not found in {:?}", attr, self)),
        }
    }
    pub fn setattr(&self, attr: &RcStr, value: Value) -> Result<()> {
        match self {
            Self::Class(cls) => match cls.static_map().get(attr) {
                Some(cell) => {
                    cell.replace(value);
                }
                None => return Err(rterr!("{:?} not found in {:?}", attr, cls)),
            },
            Self::Module(module) => match module.map().get(attr) {
                Some(cell) => {
                    cell.replace(value);
                }
                None => return Err(rterr!("{:?} not found in {:?}", attr, module)),
            },
            _ => return Err(rterr!("Attribute {:?} not found in {:?}", attr, self)),
        }
        Ok(())
    }
    pub fn apply(
        &self,
        globals: &mut Globals,
        args: Vec<Value>,
        kwargs: Option<HashMap<RcStr, Value>>,
    ) -> Result<Value> {
        match self {
            Self::Function(func) => func.apply(globals, args, kwargs),
            Self::Builtin(func) => func.apply(globals, args, kwargs),
            _ => Err(rterr!("{:?} is not a function", self)),
        }
    }
    pub fn apply_method(
        &self,
        globals: &mut Globals,
        method_name: &RcStr,
        mut args: Vec<Value>,
        kwargs: Option<HashMap<RcStr, Value>>,
    ) -> Result<Value> {
        match self {
            Self::Class(cls) => match cls.static_map().get(method_name) {
                Some(method) => method.borrow().apply(globals, args, kwargs),
                None => Err(rterr!("{:?} not found in {:?}", method_name, cls)),
            },
            Self::Module(module) => match module.get(method_name) {
                Some(method) => method.apply(globals, args, kwargs),
                None => Err(rterr!("{:?} not found in {:?}", method_name, module)),
            },
            _ => {
                let cls = self.get_class(globals).clone();
                match cls.map().get(method_name) {
                    Some(method) => {
                        args.insert(0, self.clone());
                        method.apply(globals, args, kwargs)
                    }
                    None => Err(rterr!(
                        "Method {:?} not found for instance of {:?}",
                        method_name,
                        cls
                    )),
                }
            }
        }
    }
    pub fn unpack(self, _globals: &mut Globals) -> Result<Vec<Value>> {
        match self {
            Self::List(list) => match Rc::try_unwrap(list) {
                Ok(list) => Ok(list.into_inner()),
                Err(list) => Ok(list.borrow().clone()),
            },
            _ => Err(rterr!("{:?} is not unpackable", self)),
        }
    }
}
