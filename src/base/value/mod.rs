mod cls;
mod coll;
mod conv;
mod cv;
mod format;
mod func;
mod gen;
mod hnd;
mod key;
mod m;
mod num;
mod promise;
mod strs;
mod table;
mod unpack;
mod xrefm;
use crate::Code;
use crate::Error;
use crate::Frame;
use crate::FunctionKind;
use crate::Globals;
use crate::IndexMap;
use crate::IndexSet;
use crate::Mark;
use crate::RcStr;
use crate::Result;
use std::any::Any;
use std::borrow::Cow;
use std::cell::Ref;
use std::cell::RefCell;
use std::cell::RefMut;
use std::cmp;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt;
use std::iter::FromIterator;
use std::marker::PhantomData;
use std::rc::Rc;
use std::rc::Weak;

pub use cls::*;
pub use coll::*;
pub use conv::*;
pub use cv::*;
pub use func::*;
pub use gen::*;
pub use hnd::*;
pub use key::*;
pub use m::*;
pub use promise::*;
pub use table::*;
pub use xrefm::*;

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
    Table(Rc<Table>),
    Function(Rc<Function>),
    NativeFunction(Rc<NativeFunction>),
    Generator(Rc<RefCell<Generator>>),
    NativeGenerator(Rc<RefCell<NativeGenerator>>),
    Promise(Rc<RefCell<Promise>>),
    Class(Rc<Class>),
    Module(Rc<Module>),
    Handle(Rc<HandleData>),
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
            | Self::Promise(_)
            | Self::Class(_)
            | Self::Module(_)
            | Self::Handle(_) => true,
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
            Self::Promise(_) => "Promise".into(),
            Self::Class(m) => format!("{:?}", m).into(),
            Self::Module(m) => format!("{:?}", m).into(),
            Self::Handle(m) => format!("<{} handle>", m.typename()).into(),
        }
    }
    fn terr(&self, etype: &str) -> Error {
        rterr!("Expected {} but got {}", etype, self.debug_typename())
    }
    pub fn is(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Invalid, Self::Invalid) => true,
            (Self::Nil, Self::Nil) => true,
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Number(a), Self::Number(b)) => a == b,
            (Self::String(a), Self::String(b)) => a.as_ptr() == b.as_ptr(),
            (Self::List(a), Self::List(b)) => Rc::as_ptr(a) == Rc::as_ptr(b),
            (Self::Set(a), Self::Set(b)) => Rc::as_ptr(a) == Rc::as_ptr(b),
            (Self::Map(a), Self::Map(b)) => Rc::as_ptr(a) == Rc::as_ptr(b),
            (Self::Table(a), Self::Table(b)) => Rc::as_ptr(a) == Rc::as_ptr(b),
            (Self::Function(a), Self::Function(b)) => Rc::as_ptr(a) == Rc::as_ptr(b),
            (Self::NativeFunction(a), Self::NativeFunction(b)) => Rc::as_ptr(a) == Rc::as_ptr(b),
            (Self::Generator(a), Self::Generator(b)) => Rc::as_ptr(a) == Rc::as_ptr(b),
            (Self::NativeGenerator(a), Self::NativeGenerator(b)) => Rc::as_ptr(a) == Rc::as_ptr(b),
            (Self::Promise(a), Self::Promise(b)) => Rc::as_ptr(a) == Rc::as_ptr(b),
            (Self::Class(a), Self::Class(b)) => Rc::as_ptr(a) == Rc::as_ptr(b),
            (Self::Module(a), Self::Module(b)) => Rc::as_ptr(a) == Rc::as_ptr(b),
            (Self::Handle(a), Self::Handle(b)) => Rc::as_ptr(a) == Rc::as_ptr(b),
            _ => false,
        }
    }
    pub fn in_(&self, globals: &mut Globals, other: &Self) -> Result<bool> {
        Ok(other
            .apply_method(globals, "__contains", vec![self.clone()], None)?
            .truthy())
    }
    pub fn lt(&self, other: &Self) -> Result<bool> {
        match self.partial_cmp(other) {
            Some(ord) => Ok(matches!(ord, cmp::Ordering::Less)),
            None => Err(rterr!(
                "{} and {} are not comparable",
                self.debug_typename(),
                other.debug_typename()
            )),
        }
    }
    pub fn is_nil(&self) -> bool {
        matches!(self, Value::Nil)
    }
    pub fn bool(&self) -> Result<bool> {
        if let Self::Bool(x) = self {
            Ok(*x)
        } else {
            Err(self.terr("bool"))
        }
    }
    pub fn number(&self) -> Result<f64> {
        if let Self::Number(x) = self {
            Ok(*x)
        } else {
            Err(self.terr("number"))
        }
    }
    pub fn string(&self) -> Result<&RcStr> {
        if let Self::String(x) = self {
            Ok(x)
        } else {
            Err(self.terr("string"))
        }
    }
    pub fn into_string(self) -> Result<RcStr> {
        if let Self::String(x) = self {
            Ok(x)
        } else {
            Err(self.terr("string"))
        }
    }
    pub fn list(&self) -> Result<&Rc<List>> {
        if let Self::List(x) = self {
            Ok(x)
        } else {
            Err(self.terr("list"))
        }
    }
    pub fn into_list(self) -> Result<Rc<List>> {
        if let Self::List(x) = self {
            Ok(x)
        } else {
            Err(self.terr("list"))
        }
    }
    pub fn set(&self) -> Result<&Rc<Set>> {
        if let Self::Set(x) = self {
            Ok(x)
        } else {
            Err(self.terr("set"))
        }
    }
    pub fn into_set(self) -> Result<Rc<Set>> {
        if let Self::Set(x) = self {
            Ok(x)
        } else {
            Err(self.terr("set"))
        }
    }
    pub fn map(&self) -> Result<&Rc<Map>> {
        if let Self::Map(x) = self {
            Ok(x)
        } else {
            Err(self.terr("map"))
        }
    }
    pub fn into_map(self) -> Result<Rc<Map>> {
        if let Self::Map(x) = self {
            Ok(x)
        } else {
            Err(self.terr("map"))
        }
    }
    pub fn function(&self) -> Result<&Rc<Function>> {
        if let Self::Function(func) = self {
            Ok(func)
        } else {
            Err(self.terr("function"))
        }
    }
    pub fn is_function(&self) -> bool {
        matches!(self, Self::Function(_))
    }
    pub fn into_function(self) -> Result<Rc<Function>> {
        if let Self::Function(func) = self {
            Ok(func)
        } else {
            Err(self.terr("function"))
        }
    }
    pub fn native_function(&self) -> Result<&Rc<NativeFunction>> {
        if let Self::NativeFunction(func) = self {
            Ok(func)
        } else {
            Err(self.terr("native_function"))
        }
    }
    pub fn is_native_function(&self) -> bool {
        matches!(self, Self::NativeFunction(_))
    }
    pub fn into_native_function(self) -> Result<Rc<NativeFunction>> {
        if let Self::NativeFunction(func) = self {
            Ok(func)
        } else {
            Err(self.terr("native_function"))
        }
    }
    pub fn promise(&self) -> Result<&Rc<RefCell<Promise>>> {
        if let Self::Promise(promise) = self {
            Ok(promise)
        } else {
            Err(self.terr("promise"))
        }
    }
    pub fn into_promise(self) -> Result<Rc<RefCell<Promise>>> {
        if let Self::Promise(promise) = self {
            Ok(promise)
        } else {
            Err(self.terr("promise"))
        }
    }
    pub fn class(&self) -> Result<&Rc<Class>> {
        if let Self::Class(cls) = self {
            Ok(cls)
        } else {
            Err(self.terr("class"))
        }
    }
    pub fn into_class(self) -> Result<Rc<Class>> {
        if let Self::Class(cls) = self {
            Ok(cls)
        } else {
            Err(self.terr("class"))
        }
    }
    pub fn module(&self) -> Result<&Rc<Module>> {
        if let Self::Module(m) = self {
            Ok(m)
        } else {
            Err(self.terr("module"))
        }
    }
    pub fn into_module(self) -> Result<Rc<Module>> {
        if let Self::Module(m) = self {
            Ok(m)
        } else {
            Err(self.terr("module"))
        }
    }
    pub fn into_handle<T: Any>(self) -> Result<Handle<T>> {
        if let Self::Handle(data) = self {
            HandleData::downcast(data)
        } else {
            Err(self.terr(&format!("{} handle", std::any::type_name::<T>())))
        }
    }
    pub fn is_handle<T: Any>(&self) -> bool {
        if let Self::Handle(data) = self {
            data.is::<T>()
        } else {
            false
        }
    }
    pub fn unwrap_handle<T: Any>(self) -> Result<T> {
        self.into_handle::<T>()?.unwrap()
    }
    pub fn unwrap_or_clone_handle<T: Any + Clone>(self) -> Result<T> {
        Ok(self.into_handle::<T>()?.unwrap_or_clone())
    }
    pub fn convert_to_int(self) -> Result<i64> {
        match self {
            Self::Bool(b) => Ok(if b { 1 } else { 0 }),
            Self::Number(x) => Ok(x as i64),
            Self::String(r) => Ok(r.parse::<i64>()?),
            x => Err(rterr!("Could not convert {:?} into int", x)),
        }
    }
    pub fn convert_to_float(self) -> Result<f64> {
        match self {
            Self::Bool(b) => Ok(if b { 1.0 } else { 0.0 }),
            Self::Number(x) => Ok(x),
            Self::String(r) => Ok(r.parse::<f64>()?),
            x => Err(rterr!("Could not convert {:?} into float", x)),
        }
    }
    pub fn convert_to_rcstr(self) -> RcStr {
        match self {
            Self::String(r) => r,
            Self::Handle(handle) if handle.cls().behavior().str().is_some() => {
                let handler = handle.cls().behavior().str().as_ref().unwrap().clone();
                handler(Self::Handle(handle))
            }
            _ => format!("{}", self).into(),
        }
    }
    pub fn unwrap_or_clone_string(self) -> Result<String> {
        if let Self::String(r) = self {
            Ok(r.unwrap_or_clone())
        } else {
            Err(self.terr("string"))
        }
    }
    pub fn iter(self, _globals: &mut Globals) -> Result<Self> {
        match self {
            Self::List(list) => Ok(List::generator(list).into()),
            Self::Set(set) => Ok(Set::generator(set).into()),
            Self::Map(map) => Ok(Map::generator(map).into()),
            Self::Generator(_) | Self::NativeGenerator(_) => Ok(self),
            _ => Err(self.terr("iterable")),
        }
    }
    pub fn get_class<'a>(&'a self, globals: &'a Globals) -> &'a Rc<Class> {
        globals.class_manager().get_class(self)
    }
    pub(crate) fn getattr_opt(&self, globals: &mut Globals, attr: &RcStr) -> Result<Option<Value>> {
        match self {
            Self::Table(table) => match table.map().get(attr).map(|cell| cell.borrow().clone()) {
                Some(attr) => Ok(Some(attr)),
                None => match table.cls().get_getter(attr) {
                    Some(getter) => Ok(Some(getter.apply(globals, vec![self.clone()], None)?)),
                    None => Ok(None),
                },
            },
            Self::Class(cls) => Ok(cls.static_map().get(attr).cloned()),
            Self::Module(module) => Ok(module.get(attr)),
            Self::Handle(handle) if handle.cls().behavior().getattr().is_some() => {
                let getattr = handle.cls().behavior().getattr().as_ref().unwrap();
                Ok(getattr(globals, self.clone(), attr.str())?)
            }
            _ => Ok(None),
        }
    }
    pub fn getattr(&self, globals: &mut Globals, attr: &RcStr) -> Result<Value> {
        match self.getattr_opt(globals, attr)? {
            Some(value) => Ok(value),
            None => Err(rterr!("Attribute {:?} not found in {:?}", attr, self)),
        }
    }
    pub fn getattrs(&self) -> Vec<RcStr> {
        match self {
            Self::Class(cls) => {
                let mut keys = cls
                    .static_map()
                    .keys()
                    .map(Clone::clone)
                    .collect::<Vec<_>>();
                keys.sort();
                keys
            }
            Self::Module(module) => {
                let mut keys = module.map().keys().map(Clone::clone).collect::<Vec<_>>();
                keys.sort();
                keys
            }
            Self::Table(table) => {
                let mut keys = table.map().keys().map(Clone::clone).collect::<Vec<_>>();
                keys.sort();
                keys
            }
            _ => vec![],
        }
    }
    pub fn setattr(&self, globals: &mut Globals, attr: &RcStr, value: Value) -> Result<()> {
        // We disallow setting attributes this way for both classes and modules
        //   A class's static fields are immutable
        //   A module's fields are mutable, but it should only be possible to
        //     edit a module's field from within the module itself. So
        //     trying to modify a field with setattr should be disallowed
        match self {
            Self::Table(table) => {
                if let Err(value) = table.set_opt(attr, value) {
                    if let Some(setter) = table.cls().get_setter(attr) {
                        setter.apply(globals, vec![self.clone(), value], None)?;
                    } else {
                        return Err(rterr!("Attribute {:?} not found in {:?}", attr, self));
                    }
                }
            }
            Self::Handle(handle) if handle.cls().behavior().setattr().is_some() => {
                let setattr = handle.cls().behavior().setattr().as_ref().unwrap();
                setattr(globals, self.clone(), attr.str(), value)?;
            }
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
    pub fn apply_method<M>(
        &self,
        globals: &mut Globals,
        method_name: &M,
        mut args: Vec<Value>,
        kwargs: Option<HashMap<RcStr, Value>>,
    ) -> Result<Value>
    where
        M: std::hash::Hash + std::cmp::Eq + std::fmt::Debug + ?Sized + AsRef<str>,
        RcStr: std::borrow::Borrow<M>,
    {
        match self {
            Self::Class(cls) => match cls.static_map().get(method_name) {
                Some(method) => method.apply(globals, args, kwargs),
                None => Err(rterr!("{:?} not found in {:?}", method_name, cls)),
            },
            Self::Module(module) => match module.get(method_name) {
                Some(method) => method.apply(globals, args, kwargs),
                None => Err(rterr!("{:?} not found in {:?}", method_name, module)),
            },
            Self::Handle(handle) if handle.cls().behavior().method_call().is_some() => {
                let method_call = handle.cls().behavior().method_call().as_ref().unwrap();
                method_call(globals, self.clone(), method_name.as_ref(), args, kwargs)
            }
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
    pub fn resume(&self, globals: &mut Globals, arg: Value) -> ResumeResult {
        match self {
            Self::Generator(gen) => gen.borrow_mut().resume(globals, arg),
            Self::NativeGenerator(gen) => gen.borrow_mut().resume(globals, arg),
            _ => ResumeResult::Err(rterr!("{:?} is not a generator", self)),
        }
    }
    pub fn to_index(&self, len: usize) -> Result<usize> {
        let len = len as i64;
        let i = i64::try_from(self)?;
        let adjusted = if i < 0 { i + len } else { i };
        if adjusted < 0 || adjusted >= len {
            Err(rterr!(
                "Index out of bounds (i = {} -> {}, len = {})",
                i,
                adjusted,
                len
            ))
        } else {
            Ok(adjusted as usize)
        }
    }
    pub fn to_slice_index(&self, len: usize) -> Result<usize> {
        let len = len as i64;
        let i = i64::try_from(self)?;
        let adjusted = if i < 0 { i + len } else { i };
        Ok(if adjusted < 0 {
            0
        } else if adjusted > len {
            len as usize
        } else {
            adjusted as usize
        })
    }
    pub fn to_start_index(&self, len: usize) -> Result<usize> {
        if self.is_nil() {
            Ok(0)
        } else {
            self.to_slice_index(len)
        }
    }
    pub fn to_end_index(&self, len: usize) -> Result<usize> {
        if self.is_nil() {
            Ok(len)
        } else {
            self.to_slice_index(len)
        }
    }
    pub fn getitem(&self, globals: &mut Globals, index: Value) -> Result<Value> {
        match self {
            Self::List(list) => {
                let list = list.borrow();
                let i = index.to_index(list.len())?;
                Ok(list[i].clone())
            }
            Self::Map(map) => {
                let map = map.borrow();
                let key = Key::try_from(index)?;
                match map.get(&key) {
                    Some(value) => Ok(value.clone()),
                    None => Err(rterr!("Key {:?} not found in map", key)),
                }
            }
            _ => self.apply_method(globals, "__getitem", vec![index.clone()], None),
        }
    }
    pub fn setitem(&self, globals: &mut Globals, index: Value, value: Value) -> Result<()> {
        match self {
            Self::List(list) => {
                let mut list = list.borrow_mut();
                let i = index.to_index(list.len())?;
                list[i] = value;
            }
            Self::Map(map) => {
                let mut map = map.borrow_mut();
                let key = Key::try_from(index)?;
                map.insert(key, value);
            }
            _ => {
                self.apply_method(globals, "__setitem", vec![index.clone()], None)?;
            }
        }
        Ok(())
    }
}

impl cmp::PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        match (self, other) {
            (Self::Nil, Self::Nil) => Some(cmp::Ordering::Equal),
            (Self::Bool(a), Self::Bool(b)) => a.partial_cmp(b),
            (Self::Number(a), Self::Number(b)) => a.partial_cmp(b),
            (Self::String(a), Self::String(b)) => a.partial_cmp(b),
            (Self::List(a), Self::List(b)) => a.partial_cmp(b),
            (Self::Set(a), Self::Set(b)) => a.partial_cmp(b),
            (Self::Map(a), Self::Map(b)) => a.partial_cmp(b),
            (a, b) if a == b => Some(cmp::Ordering::Equal),
            _ => None,
        }
    }
}
