mod cls;
mod coll;
mod table;
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
use std::borrow::Cow;
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
pub use table::*;
pub use conv::*;
pub use cv::*;
pub use func::*;
pub use gen::*;
pub use key::*;
pub use m::*;

#[derive(Clone, PartialEq, PartialOrd)]
pub enum Value {
    Invalid,
    Nil,
    Bool(bool),
    Number(f64),
    String(RcStr),
    List(Rc<List>),
    Set(Rc<Set>),
    Map(Rc<Map>),
    Table(Rc<Table>),
    Function(Rc<Function>),
    NativeFunction(Rc<NativeFunction>),
    Generator(Rc<RefCell<Generator>>),
    NativeGenerator(Rc<RefCell<NativeGenerator>>),
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
            Self::Table(_)
            | Self::Function(_)
            | Self::NativeFunction(_)
            | Self::Generator(_)
            | Self::NativeGenerator(_)
            | Self::Class(_)
            | Self::Module(_) => true,
        }
    }
    pub fn debug_typename(&self) -> RcStr {
        match self {
            Self::Invalid => panic!("Value::Invalid::debug_typename"),
            Self::Nil => "Nil".into(),
            Self::Bool(_) => "Bool".into(),
            Self::Number(_) => "Number".into(),
            Self::String(_) => "String".into(),
            Self::List(_) => "List".into(),
            Self::Set(_) => "Set".into(),
            Self::Map(_) => "Map".into(),
            Self::Table(obj) => obj.cls().name().clone(),
            Self::Function(_) => "Function".into(),
            Self::NativeFunction(_) => "NativeFunction".into(),
            Self::Generator(_) => "Generator".into(),
            Self::NativeGenerator(_) => "NativeGenerator".into(),
            Self::Class(m) => format!("{:?}", m).into(),
            Self::Module(m) => format!("{:?}", m).into(),
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
    pub fn iter(self, _globals: &mut Globals) -> Result<Self> {
        match self {
            Self::List(list) => Ok(List::generator(list).into()),
            Self::Set(set) => Ok(Set::generator(set).into()),
            Self::Map(map) => Ok(Map::generator(map).into()),
            Self::Generator(_) | Self::NativeGenerator(_) => Ok(self),
            _ => Err(rterr!(
                "Expected iterable but got {}",
                self.debug_typename()
            )),
        }
    }
    pub fn get_class<'a>(&'a self, globals: &'a Globals) -> &'a Rc<Class> {
        globals.class_manager().get_class(self)
    }
    pub fn getattr_opt(&self, attr: &RcStr) -> Option<Value> {
        match self {
            Self::Class(cls) => cls.static_map().get(attr).cloned(),
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
    pub fn setattr(&self, attr: &RcStr, _value: Value) -> Result<()> {
        // We disallow setting attributes this way for both classes and modules
        //   A class's static fields are immutable
        //   A module's fields are mutable, but it should only be possible to
        //     edit a module's field from within the module itself. So
        //     trying to modify a field with setattr should be disallowed
        match self {
            _ => return Err(rterr!("Attribute {:?} not found in {:?}", attr, self)),
        }
        #[allow(unreachable_code)]
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
            Self::NativeFunction(func) => func.apply(globals, args, kwargs),
            Self::Class(cls) => match cls.get_call() {
                Some(func) => func.apply(globals, args, kwargs),
                None => Err(rterr!(
                    "{:?} is not callable (the class has no static __call method)",
                    cls
                )),
            },
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
                Some(method) => method.apply(globals, args, kwargs),
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
    pub fn unpack_into_set(self, globals: &mut Globals) -> Result<IndexSet<Key>> {
        match self {
            Self::Set(set) => match Rc::try_unwrap(set) {
                Ok(set) => Ok(set.into_inner()),
                Err(set) => Ok(set.borrow().clone()),
            },
            _ => self
                .unpack(globals)?
                .into_iter()
                .map(Key::try_from)
                .collect(),
        }
    }
    pub fn unpack_into_map(self, globals: &mut Globals) -> Result<IndexMap<Key, Value>> {
        match self {
            Self::Map(map) => match Rc::try_unwrap(map) {
                Ok(map) => Ok(map.into_inner()),
                Err(map) => Ok(map.borrow().clone()),
            },
            _ => self
                .unpack(globals)?
                .into_iter()
                .map(|pairval| {
                    let [key, val] = pairval.unpack_2(globals)?;
                    Ok((Key::try_from(key)?, val))
                })
                .collect(),
        }
    }
    pub fn unpack(self, globals: &mut Globals) -> Result<Vec<Value>> {
        match self {
            Self::List(list) => match Rc::try_unwrap(list) {
                Ok(list) => Ok(list.into_inner()),
                Err(list) => Ok(list.borrow().clone()),
            },
            Self::Generator(gen) => gen.borrow_mut().unpack(globals),
            Self::NativeGenerator(gen) => gen.borrow_mut().unpack(globals),
            _ => Err(rterr!("{:?} is not unpackable", self)),
        }
    }
    pub fn unpack_2(self, globals: &mut Globals) -> Result<[Value; 2]> {
        let vec = self.unpack(globals)?;
        if vec.len() != 2 {
            Err(rterr!("Expected {} elements but got {}", 2, vec.len()))
        } else {
            let mut iter = vec.into_iter();
            Ok([iter.next().unwrap(), iter.next().unwrap()])
        }
    }
    pub fn resume(&self, globals: &mut Globals, arg: Value) -> ResumeResult {
        match self {
            Self::Generator(gen) => gen.borrow_mut().resume(globals, arg),
            Self::NativeGenerator(gen) => gen.borrow_mut().resume(globals, arg),
            _ => ResumeResult::Err(rterr!("{:?} is not a generator", self)),
        }
    }
}
