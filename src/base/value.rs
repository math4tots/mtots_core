use crate::short_name_from_full_name;
use crate::Code;
use crate::ErrorIndicator;
use crate::EvalError;
use crate::EvalResult;
use crate::Exception;
use crate::ExceptionKind;
use crate::Frame;
use crate::GeneratorResult;
use crate::Globals;
use crate::HMap;
use crate::RcPath;
use crate::RcStr;
use crate::Symbol;
use crate::VMap;
use crate::VSet;
use std::any::Any;
use std::cell::Ref;
use std::cell::RefCell;
use std::cell::RefMut;
use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum Value {
    Uninitialized,
    Nil,
    Bool(bool),
    Number(f64),
    Symbol(Symbol),
    String(RcStr),
    Bytes(Rc<Vec<u8>>),
    Path(RcPath),
    List(Rc<Vec<Value>>), // [x, ..]
    Table(Rc<Table>),     // Table(k=v, ..)
    Set(Rc<VSet>),        // Set([x, ..])
    Map(Rc<VMap>),        // [k:v, ..]
    UserObject(Rc<UserObject>),
    Exception(Rc<Exception>),
    NativeFunction(Rc<NativeFunction>),
    NativeClosure(Rc<NativeClosure>),
    Code(Rc<Code>),
    Function(Rc<Function>),
    Class(Rc<Class>),
    ExceptionKind(Rc<ExceptionKind>),

    // semi-mutable values
    NativeIterator(Rc<RefCell<NativeIterator>>),
    GeneratorObject(Rc<RefCell<GeneratorObject>>),
    Module(Rc<Module>),
    Handle(Rc<HandleData>),

    // mutable values
    MutableString(Rc<RefCell<String>>),   // @".."
    MutableBytes(Rc<RefCell<Vec<u8>>>),   // MutableBytes([..])
    MutableList(Rc<RefCell<Vec<Value>>>), // @[x, ..]
    MutableSet(Rc<RefCell<VSet>>),        // MutableSet([x, ..])
    MutableMap(Rc<RefCell<VMap>>),        // @[k:v, ..]
    MutableUserObject(Rc<MutableUserObject>),

    // 'internal' values
    Cell(Rc<RefCell<Value>>), // for implementing closures
}

impl Value {
    pub fn from_i64(i: i64) -> Self {
        Self::Number(i as f64)
    }

    pub fn from_f64(f: f64) -> Self {
        Self::Number(f)
    }

    pub fn is(&self, other: &Value) -> bool {
        fn ptr<T: ?Sized>(p: &Rc<T>) -> *const T {
            let p: &T = &*p;
            p as *const T
        }
        match (self, other) {
            (Value::Uninitialized, Value::Uninitialized) => true,
            (Value::Nil, Value::Nil) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Symbol(a), Value::Symbol(b)) => a == b,
            (Value::String(a), Value::String(b)) => a.as_ptr() == b.as_ptr(),
            (Value::Bytes(a), Value::Bytes(b)) => ptr(a) == ptr(b),
            (Value::Path(a), Value::Path(b)) => a.is_ptr_eq(b),
            (Value::List(a), Value::List(b)) => ptr(a) == ptr(b),
            (Value::Set(a), Value::Set(b)) => ptr(a) == ptr(b),
            (Value::Table(a), Value::Table(b)) => ptr(a) == ptr(b),
            (Value::Map(a), Value::Map(b)) => ptr(a) == ptr(b),
            (Value::UserObject(a), Value::UserObject(b)) => ptr(a) == ptr(b),
            (Value::Exception(a), Value::Exception(b)) => ptr(a) == ptr(b),
            (Value::NativeFunction(a), Value::NativeFunction(b)) => ptr(a) == ptr(b),
            (Value::NativeClosure(a), Value::NativeClosure(b)) => ptr(a) == ptr(b),
            (Value::Code(a), Value::Code(b)) => ptr(a) == ptr(b),
            (Value::Function(a), Value::Function(b)) => ptr(a) == ptr(b),
            (Value::Class(a), Value::Class(b)) => ptr(a) == ptr(b),
            (Value::ExceptionKind(a), Value::ExceptionKind(b)) => ptr(a) == ptr(b),
            (Value::GeneratorObject(a), Value::GeneratorObject(b)) => ptr(a) == ptr(b),
            (Value::Module(a), Value::Module(b)) => ptr(a) == ptr(b),
            (Value::Handle(a), Value::Handle(b)) => ptr(a) == ptr(b),
            (Value::MutableString(a), Value::MutableString(b)) => ptr(a) == ptr(b),
            (Value::MutableBytes(a), Value::MutableBytes(b)) => ptr(a) == ptr(b),
            (Value::MutableList(a), Value::MutableList(b)) => ptr(a) == ptr(b),
            (Value::MutableSet(a), Value::MutableSet(b)) => ptr(a) == ptr(b),
            (Value::MutableMap(a), Value::MutableMap(b)) => ptr(a) == ptr(b),
            (Value::Cell(a), Value::Cell(b)) => ptr(a) == ptr(b),
            _ => false,
        }
    }

    pub fn is_nil(&self) -> bool {
        if let Value::Nil = self {
            true
        } else {
            false
        }
    }

    pub fn bool(&self) -> Option<bool> {
        if let Value::Bool(x) = self {
            Some(*x)
        } else {
            None
        }
    }

    pub fn int(&self) -> Option<i64> {
        match self {
            Value::Number(x) if x.fract() == 0.0 => Some(*x as i64),
            _ => None,
        }
    }

    pub fn float(&self) -> Option<f64> {
        if let Value::Number(x) = self {
            Some(*x)
        } else {
            None
        }
    }

    pub fn floatlike(&self) -> Option<f64> {
        match self {
            Value::Number(x) => Some(*x),
            _ => None,
        }
    }

    pub fn symbol(&self) -> Option<Symbol> {
        match self {
            Value::Symbol(x) => Some(*x),
            _ => None,
        }
    }

    pub fn string(&self) -> Option<&RcStr> {
        if let Value::String(x) = self {
            Some(x)
        } else {
            None
        }
    }

    pub fn path(&self) -> Option<&RcPath> {
        if let Value::Path(x) = self {
            Some(x)
        } else {
            None
        }
    }

    pub fn pathlike(&self) -> Option<&Path> {
        match self {
            Value::String(x) => Some(Path::new(&**x)),
            Value::Path(x) => Some(x),
            _ => None,
        }
    }

    pub fn list(&self) -> Option<&Rc<Vec<Value>>> {
        if let Value::List(x) = self {
            Some(x)
        } else {
            None
        }
    }

    pub fn table(&self) -> Option<&Rc<Table>> {
        if let Value::Table(x) = self {
            Some(x)
        } else {
            None
        }
    }

    pub fn set(&self) -> Option<&Rc<VSet>> {
        if let Value::Set(x) = self {
            Some(x)
        } else {
            None
        }
    }

    pub fn map(&self) -> Option<&Rc<VMap>> {
        if let Value::Map(x) = self {
            Some(x)
        } else {
            None
        }
    }

    pub fn module(&self) -> Option<&Rc<Module>> {
        if let Value::Module(x) = self {
            Some(x)
        } else {
            None
        }
    }

    pub fn mutable_string(&self) -> Option<&Rc<RefCell<String>>> {
        if let Value::MutableString(x) = self {
            Some(x)
        } else {
            None
        }
    }

    pub fn mutable_list(&self) -> Option<&Rc<RefCell<Vec<Value>>>> {
        if let Value::MutableList(x) = self {
            Some(x)
        } else {
            None
        }
    }

    pub fn mutable_set(&self) -> Option<&Rc<RefCell<VSet>>> {
        if let Value::MutableSet(x) = self {
            Some(x)
        } else {
            None
        }
    }

    pub fn mutable_map(&self) -> Option<&Rc<RefCell<VMap>>> {
        if let Value::MutableMap(x) = self {
            Some(x)
        } else {
            None
        }
    }

    pub fn cell(&self) -> Option<&Rc<RefCell<Value>>> {
        if let Value::Cell(x) = self {
            Some(x)
        } else {
            None
        }
    }

    pub fn is_handle<T: Any>(&self) -> bool {
        if let Value::Handle(data) = self {
            data.value.borrow().is::<T>()
        } else {
            false
        }
    }

    pub fn handle<T: Any>(&self) -> Option<Handle<T>> {
        if let Value::Handle(data) = self {
            if data.value.borrow().is::<T>() {
                Some(Handle(data.clone(), PhantomData))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn handle_borrow<T: Any>(&self) -> Option<Ref<T>> {
        if let Value::Handle(data) = self {
            if data.value.borrow().is::<T>() {
                Some(Ref::map(data.value.borrow(), |bx| {
                    bx.downcast_ref().unwrap()
                }))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn handle_borrow_mut<T: Any>(&self) -> Option<RefMut<T>> {
        if let Value::Handle(data) = self {
            if data.value.borrow().is::<T>() {
                Some(RefMut::map(data.value.borrow_mut(), |bx| {
                    bx.downcast_mut().unwrap()
                }))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn into_handle<T: Any>(self) -> std::result::Result<Handle<T>, Self> {
        if let Value::Handle(data) = self {
            if data.value.borrow().is::<T>() {
                Ok(Handle(data, PhantomData))
            } else {
                Err(Value::Handle(data))
            }
        } else {
            Err(self)
        }
    }

    pub fn kind(&self) -> ValueKind {
        match self {
            Value::Uninitialized => ValueKind::Uninitialized,
            Value::Nil => ValueKind::Nil,
            Value::Bool(..) => ValueKind::Bool,
            Value::Number(..) => ValueKind::Number,
            Value::Symbol(..) => ValueKind::Symbol,
            Value::String(..) => ValueKind::String,
            Value::Bytes(..) => ValueKind::Bytes,
            Value::Path(..) => ValueKind::Path,
            Value::List(..) => ValueKind::List,
            Value::Table(..) => ValueKind::Table,
            Value::Set(..) => ValueKind::Set,
            Value::Map(..) => ValueKind::Map,
            Value::UserObject(..) => ValueKind::UserObject,
            Value::Exception(..) => ValueKind::Exception,
            Value::NativeFunction(..) => ValueKind::NativeFunction,
            Value::NativeClosure(..) => ValueKind::NativeClosure,
            Value::Module(..) => ValueKind::Module,
            Value::Handle(handle) => ValueKind::Handle(handle.type_name),
            Value::Code(..) => ValueKind::Code,
            Value::Function(..) => ValueKind::Function,
            Value::Class(..) => ValueKind::Class,
            Value::ExceptionKind(..) => ValueKind::ExceptionKind,
            Value::NativeIterator(..) => ValueKind::NativeIterator,
            Value::GeneratorObject(..) => ValueKind::GeneratorObject,
            Value::MutableString(..) => ValueKind::MutableString,
            Value::MutableBytes(..) => ValueKind::MutableBytes,
            Value::MutableList(..) => ValueKind::MutableList,
            Value::MutableSet(..) => ValueKind::MutableSet,
            Value::MutableMap(..) => ValueKind::MutableMap,
            Value::MutableUserObject(..) => ValueKind::MutableUserObject,
            Value::Cell(..) => ValueKind::Cell,
        }
    }
}

impl Default for Value {
    fn default() -> Value {
        Value::Uninitialized
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Bool(x) => write!(f, "{}", if *x { "true" } else { "false" }),
            Value::Number(x) => write!(f, "{}", x),
            Value::Symbol(x) => write!(f, "{}", x),
            Value::String(s) => write!(f, "{}", s),
            Value::Bytes(s) => write!(f, "Bytes({:?})", s),
            Value::Path(p) => write!(f, "{:?}", p),
            Value::List(list) => {
                write!(f, "[")?;
                let mut first = true;
                for x in list.iter() {
                    if !first {
                        write!(f, ", ")?;
                    }
                    fmt::Display::fmt(x, f)?;
                    first = false;
                }
                write!(f, "]")
            }
            Value::Class(cls) => write!(f, "{:?}", cls),
            Value::Module(m) => write!(f, "{:?}", m),
            Value::Handle(handle) => write!(f, "{:?}", handle),
            _ => fmt::Debug::fmt(self, f),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueKind {
    Uninitialized,
    Nil,
    Bool,
    Int,
    Number,
    Symbol,
    String,
    Bytes,
    Path,
    List,
    Table,
    Set,
    Map,
    UserObject,
    Exception,
    NativeFunction,
    NativeClosure,
    Code,
    Function,
    Class,
    ExceptionKind,
    NativeIterator,
    GeneratorObject,
    Module,
    Handle(&'static str),
    MutableString,
    MutableBytes,
    MutableList,
    MutableSet,
    MutableMap,
    MutableUserObject,
    Cell,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum ConstValue {
    Uninitialized,
    Nil,
    Bool(bool),
    Float(u64),
    Symbol(Symbol),
    String(RcStr),
    Path(RcPath),
    Bytes(Rc<Vec<u8>>),
}

impl From<ConstValue> for Value {
    fn from(cv: ConstValue) -> Value {
        match cv {
            ConstValue::Uninitialized => Value::Uninitialized,
            ConstValue::Nil => Value::Nil,
            ConstValue::Bool(x) => Value::Bool(x),
            ConstValue::Float(x) => Value::Number(f64::from_bits(x)),
            ConstValue::Symbol(x) => Value::Symbol(x),
            ConstValue::String(x) => Value::String(x),
            ConstValue::Path(x) => Value::Path(x),
            ConstValue::Bytes(x) => Value::Bytes(x),
        }
    }
}

impl From<()> for ConstValue {
    fn from(_: ()) -> ConstValue {
        ConstValue::Nil
    }
}

impl From<bool> for ConstValue {
    fn from(x: bool) -> ConstValue {
        ConstValue::Bool(x)
    }
}

impl From<i64> for ConstValue {
    fn from(x: i64) -> ConstValue {
        ConstValue::Float((x as f64).to_bits())
    }
}

impl From<f64> for ConstValue {
    fn from(x: f64) -> ConstValue {
        ConstValue::Float(x.to_bits())
    }
}

impl From<Symbol> for ConstValue {
    fn from(x: Symbol) -> ConstValue {
        ConstValue::Symbol(x)
    }
}

impl From<RcStr> for ConstValue {
    fn from(x: RcStr) -> ConstValue {
        ConstValue::String(x)
    }
}

impl From<String> for ConstValue {
    fn from(x: String) -> ConstValue {
        ConstValue::String(x.into())
    }
}

impl From<&str> for ConstValue {
    fn from(x: &str) -> ConstValue {
        ConstValue::String(x.into())
    }
}

impl From<RcPath> for ConstValue {
    fn from(x: RcPath) -> ConstValue {
        ConstValue::Path(x)
    }
}

impl From<&Path> for ConstValue {
    fn from(x: &Path) -> ConstValue {
        ConstValue::Path(x.into())
    }
}

impl From<PathBuf> for ConstValue {
    fn from(x: PathBuf) -> ConstValue {
        let x: &Path = &x;
        ConstValue::Path(x.into())
    }
}

impl From<Vec<u8>> for ConstValue {
    fn from(x: Vec<u8>) -> ConstValue {
        ConstValue::Bytes(x.into())
    }
}

impl From<Rc<Vec<u8>>> for ConstValue {
    fn from(x: Rc<Vec<u8>>) -> ConstValue {
        ConstValue::Bytes(x)
    }
}

impl From<&Rc<Vec<u8>>> for ConstValue {
    fn from(x: &Rc<Vec<u8>>) -> ConstValue {
        ConstValue::Bytes(x.clone())
    }
}

impl From<()> for Value {
    fn from(_: ()) -> Value {
        Value::Nil
    }
}

impl From<bool> for Value {
    fn from(x: bool) -> Value {
        Value::Bool(x)
    }
}

impl From<isize> for Value {
    fn from(x: isize) -> Value {
        Self::Number(x as f64)
    }
}

impl From<i64> for Value {
    fn from(x: i64) -> Value {
        Self::Number(x as f64)
    }
}

impl From<i32> for Value {
    fn from(x: i32) -> Value {
        Self::Number(x as f64)
    }
}

impl From<i16> for Value {
    fn from(x: i16) -> Value {
        Self::Number(x as f64)
    }
}

impl From<i8> for Value {
    fn from(x: i8) -> Value {
        Self::Number(x as f64)
    }
}

impl From<usize> for Value {
    fn from(x: usize) -> Value {
        Self::Number(x as f64)
    }
}

impl From<u64> for Value {
    fn from(x: u64) -> Value {
        Self::Number(x as f64)
    }
}

impl From<u32> for Value {
    fn from(x: u32) -> Value {
        Self::Number(x as f64)
    }
}

impl From<u16> for Value {
    fn from(x: u16) -> Value {
        Self::Number(x as f64)
    }
}

impl From<u8> for Value {
    fn from(x: u8) -> Value {
        Self::Number(x as f64)
    }
}

impl From<f64> for Value {
    fn from(x: f64) -> Value {
        Value::Number(x)
    }
}

impl From<f32> for Value {
    fn from(x: f32) -> Value {
        Self::Number(x as f64)
    }
}

impl From<Symbol> for Value {
    fn from(x: Symbol) -> Value {
        Value::Symbol(x)
    }
}

impl From<RcStr> for Value {
    fn from(x: RcStr) -> Value {
        Value::String(x)
    }
}

impl From<String> for Value {
    fn from(x: String) -> Value {
        Value::String(x.into())
    }
}

impl From<&str> for Value {
    fn from(x: &str) -> Value {
        Value::String(x.into())
    }
}

impl From<RcPath> for Value {
    fn from(x: RcPath) -> Value {
        Value::Path(x)
    }
}

impl From<&Path> for Value {
    fn from(x: &Path) -> Value {
        Value::Path(x.into())
    }
}

impl From<PathBuf> for Value {
    fn from(x: PathBuf) -> Value {
        let x: &Path = &x;
        Value::Path(x.into())
    }
}

impl From<Vec<u8>> for Value {
    fn from(x: Vec<u8>) -> Value {
        Value::Bytes(x.into())
    }
}

impl From<Rc<Vec<u8>>> for Value {
    fn from(x: Rc<Vec<u8>>) -> Value {
        Value::Bytes(x)
    }
}

impl From<&Rc<Vec<u8>>> for Value {
    fn from(x: &Rc<Vec<u8>>) -> Value {
        Value::Bytes(x.clone())
    }
}

impl From<Vec<Value>> for Value {
    fn from(list: Vec<Value>) -> Value {
        Value::List(list.into())
    }
}

impl From<Rc<Vec<Value>>> for Value {
    fn from(list: Rc<Vec<Value>>) -> Value {
        Value::List(list)
    }
}

impl From<VSet> for Value {
    fn from(set: VSet) -> Value {
        Value::Set(set.into())
    }
}

impl From<Rc<VSet>> for Value {
    fn from(set: Rc<VSet>) -> Value {
        Value::Set(set)
    }
}

impl From<VMap> for Value {
    fn from(map: VMap) -> Value {
        Value::Map(map.into())
    }
}

impl From<Rc<VMap>> for Value {
    fn from(map: Rc<VMap>) -> Value {
        Value::Map(map)
    }
}

impl<'a> From<&'a Rc<Vec<Value>>> for Value {
    fn from(list: &'a Rc<Vec<Value>>) -> Value {
        list.clone().into()
    }
}

impl From<Rc<Class>> for Value {
    fn from(cls: Rc<Class>) -> Value {
        Value::Class(cls)
    }
}

impl From<NativeIterator> for Value {
    fn from(niter: NativeIterator) -> Value {
        Value::NativeIterator(RefCell::new(niter).into())
    }
}

impl From<Rc<RefCell<NativeIterator>>> for Value {
    fn from(niter: Rc<RefCell<NativeIterator>>) -> Value {
        Value::NativeIterator(niter)
    }
}

#[derive(Clone)]
pub struct Table {
    map: HashMap<Symbol, Value>,
}

impl fmt::Debug for Table {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.map)
    }
}

impl Table {
    pub fn new(map: HashMap<Symbol, Value>) -> Table {
        Table { map }
    }

    pub fn get(&self, symbol: Symbol) -> Option<&Value> {
        self.map.get(&symbol)
    }

    pub fn map(&self) -> &HashMap<Symbol, Value> {
        &self.map
    }

    pub fn map_move(self) -> HashMap<Symbol, Value> {
        self.map
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Symbol, &Value)> {
        self.map.iter()
    }
}

pub struct UserObject {
    cls: Rc<Class>,
    map: HashMap<Symbol, Value>,
}

impl UserObject {
    pub fn get(&self, symbol: Symbol) -> Option<&Value> {
        self.map.get(&symbol)
    }

    pub fn cls(&self) -> &Rc<Class> {
        &self.cls
    }
}

impl From<UserObject> for Value {
    fn from(obj: UserObject) -> Value {
        Value::UserObject(obj.into())
    }
}

impl From<Rc<UserObject>> for Value {
    fn from(obj: Rc<UserObject>) -> Value {
        Value::UserObject(obj)
    }
}

impl fmt::Debug for UserObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{} instance>", self.cls.full_name)
    }
}

pub struct MutableUserObject {
    cls: Rc<Class>,
    map: HashMap<Symbol, RefCell<Value>>,
}

impl MutableUserObject {
    pub fn get(&self, symbol: Symbol) -> Option<&RefCell<Value>> {
        self.map.get(&symbol)
    }

    pub fn cls(&self) -> &Rc<Class> {
        &self.cls
    }
}

impl From<MutableUserObject> for Value {
    fn from(obj: MutableUserObject) -> Value {
        Value::MutableUserObject(obj.into())
    }
}

impl From<Rc<MutableUserObject>> for Value {
    fn from(obj: Rc<MutableUserObject>) -> Value {
        Value::MutableUserObject(obj)
    }
}

impl fmt::Debug for MutableUserObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{} @instance>", self.cls.full_name)
    }
}

#[derive(Debug)]
pub enum ArgumentError {
    MismatchedArgumentCount { argc: usize, min: usize, max: usize },
    NotEnoughPositionalArguments { argc: usize },
    TooManyPositionalArguments { argc: usize },
    TooManyKeywordArguments(HashMap<Symbol, Value>),
}
impl fmt::Display for ArgumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArgumentError::MismatchedArgumentCount { argc, min, max } => {
                if min == max {
                    write!(f, "Expected {} args but got {}", min, argc)?;
                } else if *max == std::usize::MAX {
                    write!(f, "Expected at least {} args but got {}", min, argc)?;
                } else {
                    write!(
                        f,
                        "Expected between {} and {} args, but got {}",
                        min, max, argc
                    )?;
                }
            }
            ArgumentError::NotEnoughPositionalArguments { argc } => {
                write!(f, "Not enough positional args (got {})", argc)?;
            }
            ArgumentError::TooManyPositionalArguments { argc } => {
                write!(f, "Too many positional args (got {})", argc)?;
            }
            ArgumentError::TooManyKeywordArguments(map) => {
                write!(f, "Too many keyword args (")?;
                let mut first = true;
                for key in map.keys() {
                    if !first {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", key)?;
                    first = false;
                }
                write!(f, ")")?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
enum ParameterInfoKind {
    // no required or optional parameters, and variadic and keywords
    // are set. The function expects to accept the input as is
    // without any parameter processing
    PassThrough,

    /// required, *args, and **kwargs are all present, but
    /// there are no optional/default parameters.
    /// This case is almost like PassThrough, except that we
    /// have to check that the input has a minimum number of
    /// positional arguments.
    OnlyOptionalsEmpty,

    // requires full argument processing logic
    Mixed,
}
#[derive(Debug)]
pub struct ParameterInfo {
    kind: ParameterInfoKind,
    required: Vec<Symbol>,
    optional: Vec<(Symbol, Value)>,
    variadic: Option<Symbol>,
    keywords: Option<Symbol>,
}
impl ParameterInfo {
    pub fn builder() -> ParameterInfoBuilder {
        ParameterInfoBuilder::default()
    }
    pub fn new(
        required: Vec<Symbol>,
        optional: Vec<(Symbol, Value)>,
        variadic: Option<Symbol>,
        keywords: Option<Symbol>,
    ) -> ParameterInfo {
        let kind = if optional.is_empty() && variadic.is_some() && keywords.is_some() {
            if required.is_empty() {
                ParameterInfoKind::PassThrough
            } else {
                ParameterInfoKind::OnlyOptionalsEmpty
            }
        } else {
            ParameterInfoKind::Mixed
        };
        ParameterInfo {
            kind,
            required,
            optional,
            variadic,
            keywords,
        }
    }
    pub fn empty() -> ParameterInfo {
        Self::new(vec![], vec![], None, None)
    }
    pub fn pass_through() -> ParameterInfo {
        Self::new(vec![], vec![], Some(Symbol::ARGS), Some(Symbol::KWARGS))
    }
    pub fn kwargs_only() -> ParameterInfo {
        Self::new(vec![], vec![], None, Some(Symbol::KWARGS))
    }
    pub fn translate(
        &self,
        args: Vec<Value>,
        kwargs: Option<HashMap<Symbol, Value>>,
    ) -> Result<(Vec<Value>, Option<HashMap<Symbol, Value>>), ArgumentError> {
        match self.kind {
            ParameterInfoKind::PassThrough => Ok((args, kwargs)),
            ParameterInfoKind::OnlyOptionalsEmpty => {
                if args.len() < self.required().len() {
                    Err(ArgumentError::NotEnoughPositionalArguments { argc: args.len() })
                } else {
                    Ok((args, kwargs))
                }
            }
            ParameterInfoKind::Mixed => self.translate_without_kind(args, kwargs),
        }
    }
    fn translate_without_kind(
        &self,
        mut args: Vec<Value>,
        kwargs: Option<HashMap<Symbol, Value>>,
    ) -> Result<(Vec<Value>, Option<HashMap<Symbol, Value>>), ArgumentError> {
        if let Some(mut kwargs) = kwargs {
            let argc = args.len();
            let mut args = args.into_iter();
            let mut args_peek = args.next();
            let mut new_args = Vec::new();
            for name in &self.required {
                if let Some(value) = kwargs.remove(name) {
                    new_args.push(value);
                } else if let Some(value) = args_peek {
                    new_args.push(value);
                    args_peek = args.next();
                } else {
                    return Err(ArgumentError::NotEnoughPositionalArguments { argc });
                }
            }
            for (name, default_value) in &self.optional {
                if let Some(value) = kwargs.remove(name) {
                    new_args.push(value);
                } else if let Some(value) = args_peek {
                    new_args.push(value);
                    args_peek = args.next();
                } else {
                    new_args.push(default_value.clone());
                }
            }
            if let Some(peek) = args_peek {
                if self.variadic().is_some() {
                    new_args.push(peek);
                    new_args.extend(args);
                } else {
                    return Err(ArgumentError::TooManyPositionalArguments { argc });
                }
            }
            if self.keywords.is_none() {
                if kwargs.is_empty() {
                    Ok((new_args, None))
                } else {
                    Err(ArgumentError::TooManyKeywordArguments(kwargs))
                }
            } else {
                Ok((new_args, Some(kwargs)))
            }
        } else {
            let argc = args.len();
            let min = self.required.len();
            let wopt = min + self.optional.len();
            let max = if self.variadic.is_some() {
                std::usize::MAX
            } else {
                wopt
            };
            if min <= argc && argc <= max {
                if argc < wopt {
                    args.extend(self.optional[argc - min..].iter().map(|(_, v)| v.clone()));
                }
                Ok((
                    args,
                    if self.keywords.is_none() {
                        None
                    } else {
                        Some(HashMap::new())
                    },
                ))
            } else {
                Err(ArgumentError::MismatchedArgumentCount { argc, min, max })
            }
        }
    }
    pub fn required(&self) -> &Vec<Symbol> {
        &self.required
    }
    pub fn optional(&self) -> &Vec<(Symbol, Value)> {
        &self.optional
    }
    pub fn variadic(&self) -> &Option<Symbol> {
        &self.variadic
    }
    pub fn keywords(&self) -> &Option<Symbol> {
        &self.keywords
    }
}

#[derive(Default)]
pub struct ParameterInfoBuilder {
    required: Vec<Symbol>,
    optional: Vec<(Symbol, Value)>,
    variadic: Option<Symbol>,
    keywords: Option<Symbol>,
}

impl ParameterInfoBuilder {
    pub fn required<S: Into<Symbol>>(mut self, s: S) -> Self {
        self.required.push(s.into());
        self
    }

    pub fn optional<S: Into<Symbol>, V: Into<Value>>(mut self, s: S, v: V) -> Self {
        self.optional.push((s.into(), v.into()));
        self
    }

    pub fn variadic<S: Into<Symbol>>(mut self, s: S) -> Self {
        self.variadic = Some(s.into());
        self
    }

    pub fn keywords<S: Into<Symbol>>(mut self, s: S) -> Self {
        self.keywords = Some(s.into());
        self
    }

    pub fn build(self) -> ParameterInfo {
        ParameterInfo::new(self.required, self.optional, self.variadic, self.keywords)
    }
}

impl From<ParameterInfoBuilder> for ParameterInfo {
    fn from(builder: ParameterInfoBuilder) -> Self {
        builder.build()
    }
}

impl From<()> for ParameterInfo {
    fn from(_: ()) -> Self {
        Self::empty()
    }
}

impl<S: Into<Symbol> + Clone> From<&[S]> for ParameterInfo {
    fn from(reqs: &[S]) -> Self {
        Self::new(
            reqs.to_vec().into_iter().map(Into::into).collect(),
            vec![],
            None,
            None,
        )
    }
}

impl From<[&str; 0]> for ParameterInfo {
    fn from(_: [&str; 0]) -> Self {
        Self::empty()
    }
}
impl From<&[&str; 0]> for ParameterInfo {
    fn from(_: &[&str; 0]) -> Self {
        Self::empty()
    }
}
macro_rules! from_arr_for_parameter_info {
    ($n:tt) => {
        impl<S: Into<Symbol> + Clone> From<[S; $n]> for ParameterInfo {
            fn from(reqs: [S; $n]) -> Self {
                Self::new(
                    reqs.to_vec().into_iter().map(Into::into).collect(),
                    vec![],
                    None,
                    None,
                )
            }
        }
        impl<S: Into<Symbol> + Clone> From<&[S; $n]> for ParameterInfo {
            fn from(reqs: &[S; $n]) -> Self {
                Self::new(
                    reqs.to_vec().into_iter().map(Into::into).collect(),
                    vec![],
                    None,
                    None,
                )
            }
        }
    };
}

from_arr_for_parameter_info!(1);
from_arr_for_parameter_info!(2);
from_arr_for_parameter_info!(3);
from_arr_for_parameter_info!(4);
from_arr_for_parameter_info!(5);
from_arr_for_parameter_info!(6);
from_arr_for_parameter_info!(7);
from_arr_for_parameter_info!(8);
from_arr_for_parameter_info!(9);
from_arr_for_parameter_info!(10);
from_arr_for_parameter_info!(11);
from_arr_for_parameter_info!(12);
from_arr_for_parameter_info!(13);
from_arr_for_parameter_info!(14);
from_arr_for_parameter_info!(15);

pub struct DocStr(Option<RcStr>);

impl From<Option<RcStr>> for DocStr {
    fn from(opt: Option<RcStr>) -> Self {
        Self(opt)
    }
}

impl From<()> for DocStr {
    fn from(_: ()) -> Self {
        Self(None)
    }
}

impl From<&str> for DocStr {
    fn from(s: &str) -> Self {
        Self(Some(s.into()))
    }
}

impl From<String> for DocStr {
    fn from(s: String) -> Self {
        Self(Some(s.into()))
    }
}

impl From<RcStr> for DocStr {
    fn from(s: RcStr) -> Self {
        Self(Some(s))
    }
}

type FunctionResult = EvalResult<Value>;
pub type NativeFunctionBody =
    fn(&mut Globals, args: Vec<Value>, kwargs: Option<HashMap<Symbol, Value>>) -> FunctionResult;
pub struct NativeFunction {
    name: RcStr,
    parameter_info: ParameterInfo,
    doc: Option<RcStr>,
    body: NativeFunctionBody,
}
impl From<NativeFunction> for Value {
    fn from(f: NativeFunction) -> Value {
        Value::NativeFunction(f.into())
    }
}
impl From<Rc<NativeFunction>> for Value {
    fn from(f: Rc<NativeFunction>) -> Value {
        Value::NativeFunction(f)
    }
}
impl From<&Rc<NativeFunction>> for Value {
    fn from(f: &Rc<NativeFunction>) -> Value {
        Value::NativeFunction(f.clone())
    }
}
impl fmt::Debug for NativeFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<native function {}>", self.name())
    }
}
impl NativeFunction {
    /// The most direct way to create a new NativeFunction
    /// All other constructors are convenience wrappers around this one
    pub fn new<N, P, D>(name: N, parameter_info: P, doc: D, body: NativeFunctionBody) -> Self
    where
        N: Into<RcStr>,
        P: Into<ParameterInfo>,
        D: Into<DocStr>,
    {
        Self {
            name: name.into(),
            parameter_info: parameter_info.into(),
            doc: doc.into().0,
            body,
        }
    }
    pub fn name(&self) -> &RcStr {
        &self.name
    }
    pub fn parameter_info(&self) -> &ParameterInfo {
        &self.parameter_info
    }
    pub fn doc(&self) -> &Option<RcStr> {
        &self.doc
    }
    pub fn apply_with_kwargs(
        &self,
        globals: &mut Globals,
        args: Vec<Value>,
        kwargs: Option<HashMap<Symbol, Value>>,
    ) -> FunctionResult {
        let (args, kwargs) = match self.parameter_info.translate(args, kwargs) {
            Ok(pair) => pair,
            Err(error) => return globals.set_exc_legacy(error.into()),
        };
        (self.body)(globals, args, kwargs)
    }
}

// We use Fn instead of FnMut, because if the body were FnMut, all native closures
// would not be re-entrant. Closures that want to mutate closure state should use
// RefCells as needed.
pub type NativeClosureBody =
    Box<dyn Fn(&mut Globals, Vec<Value>, Option<HashMap<Symbol, Value>>) -> FunctionResult>;
pub struct NativeClosure {
    name: RcStr,
    parameter_info: ParameterInfo,
    doc: Option<RcStr>,
    body: NativeClosureBody,
}
impl From<NativeClosure> for Value {
    fn from(f: NativeClosure) -> Value {
        Value::NativeClosure(f.into())
    }
}
impl From<Rc<NativeClosure>> for Value {
    fn from(f: Rc<NativeClosure>) -> Value {
        Value::NativeClosure(f)
    }
}
impl From<&Rc<NativeClosure>> for Value {
    fn from(f: &Rc<NativeClosure>) -> Value {
        Value::NativeClosure(f.clone())
    }
}
impl fmt::Debug for NativeClosure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<native closure {}>", self.name())
    }
}
impl NativeClosure {
    pub fn new<N, P, F, D>(name: N, parameter_info: P, doc: D, body: F) -> NativeClosure
    where
        N: Into<RcStr>,
        P: Into<ParameterInfo>,
        D: Into<DocStr>,
        F: Fn(&mut Globals, Vec<Value>, Option<HashMap<Symbol, Value>>) -> FunctionResult + 'static,
    {
        NativeClosure {
            name: name.into(),
            parameter_info: parameter_info.into(),
            doc: doc.into().0,
            body: Box::new(body),
        }
    }
    pub fn name(&self) -> &RcStr {
        &self.name
    }
    pub fn doc(&self) -> &Option<RcStr> {
        &self.doc
    }
    pub fn apply_with_kwargs(
        &self,
        globals: &mut Globals,
        args: Vec<Value>,
        kwargs: Option<HashMap<Symbol, Value>>,
    ) -> FunctionResult {
        let (args, kwargs) = match self.parameter_info.translate(args, kwargs) {
            Ok(pair) => pair,
            Err(error) => return globals.set_exc_legacy(error.into()),
        };
        (self.body)(globals, args, kwargs)
    }
}

pub struct Function {
    freevar_bindings: Vec<Rc<RefCell<Value>>>,
    code: Rc<Code>,
}
impl fmt::Debug for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<function {}>", self.name())
    }
}
impl Function {
    pub fn new(freevar_bindings: Vec<Rc<RefCell<Value>>>, code: Rc<Code>) -> Function {
        Function {
            freevar_bindings,
            code,
        }
    }
    pub fn full_name(&self) -> &RcStr {
        self.code.full_name()
    }
    pub fn name(&self) -> &RcStr {
        self.code.short_name()
    }
    pub fn doc(&self) -> &Option<RcStr> {
        self.code.doc()
    }
    pub fn disasm_str(&self) -> String {
        self.code.debugstr0()
    }
    pub fn parameter_info(&self) -> &ParameterInfo {
        self.code.parameter_info()
    }
    pub fn apply_with_kwargs(
        &self,
        globals: &mut Globals,
        args: Vec<Value>,
        kwargs: Option<HashMap<Symbol, Value>>,
    ) -> FunctionResult {
        let mut frame = Frame::for_func(&self.code, self.freevar_bindings.clone());
        if let Err(error) = self.code.assign_args(&mut frame, args, kwargs) {
            return globals.set_exc_legacy(error.into());
        }
        if self.code.is_generator() {
            Ok(Value::GeneratorObject(
                RefCell::new(GeneratorObject {
                    status: GeneratorStatus::NotStarted,
                    frame: frame,
                    code: self.code.clone(),
                })
                .into(),
            ))
        } else {
            Ok(self.code.run(globals, &mut frame)?)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ClassKind {
    Trait,
    NativeClass,
    UserDefinedClass,
    UserDefinedCaseClass,
    UserDefinedMutableClass,
}
impl ClassKind {
    pub fn from_usize(i: usize) -> Option<ClassKind> {
        match i {
            0 => Some(ClassKind::Trait),
            1 => Some(ClassKind::NativeClass),
            2 => Some(ClassKind::UserDefinedClass),
            3 => Some(ClassKind::UserDefinedCaseClass),
            4 => Some(ClassKind::UserDefinedMutableClass),
            _ => None,
        }
    }

    pub fn to_usize(self) -> usize {
        match self {
            ClassKind::Trait => 0,
            ClassKind::NativeClass => 1,
            ClassKind::UserDefinedClass => 2,
            ClassKind::UserDefinedCaseClass => 3,
            ClassKind::UserDefinedMutableClass => 4,
        }
    }
}
pub struct Class {
    kind: ClassKind,
    full_name: RcStr,
    short_name: RcStr,
    doc: Option<RcStr>,
    fields: Vec<Symbol>,
    map: HashMap<Symbol, Value>,
    static_map: HashMap<Symbol, Value>,
    fields_as_parameter_info: ParameterInfo,
    has_static_call: bool,
}
impl fmt::Debug for Class {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match self.kind {
            ClassKind::Trait => "trait",
            ClassKind::NativeClass | ClassKind::UserDefinedClass => "class",
            ClassKind::UserDefinedCaseClass => "case class",
            ClassKind::UserDefinedMutableClass => "@class",
        };
        write!(f, "<{} {}>", kind, self.full_name)
    }
}
impl Class {
    /// The main way to create a new class.
    /// Use Globals::new_class
    pub(crate) fn new(
        globals: &mut Globals,
        kind: ClassKind,
        full_name: RcStr,
        mut bases: Vec<Rc<Class>>,
        doc: Option<RcStr>,
        fields: Option<Vec<Symbol>>,
        map: HashMap<Symbol, Value>,
        static_map: HashMap<Symbol, Value>,
    ) -> EvalResult<Rc<Class>> {
        if bases.is_empty() {
            bases.push(globals.builtin_classes().Object.clone());
        }
        match Self::new00(kind, full_name, bases, doc, fields, map, static_map) {
            Ok(cls) => Ok(cls),
            Err(error) => globals.set_exc_other(error.into()),
        }
    }

    /// version of new for creating builtin classes
    /// Users of this method should ensure that the created Class inherits
    /// from Object either directly or indirectly
    pub(crate) fn new0(
        kind: ClassKind,
        full_name: RcStr,
        bases: Vec<Rc<Class>>,
        doc: Option<&str>,
        map: HashMap<Symbol, Value>,
        static_map: HashMap<Symbol, Value>,
    ) -> Rc<Class> {
        Self::new00(
            kind,
            full_name,
            bases,
            doc.map(RcStr::from),
            None,
            map,
            static_map,
        )
        .unwrap()
    }
    fn new00(
        kind: ClassKind,
        full_name: RcStr,
        bases: Vec<Rc<Class>>,
        doc: Option<RcStr>,
        fields: Option<Vec<Symbol>>,
        mut map: HashMap<Symbol, Value>,
        static_map: HashMap<Symbol, Value>,
    ) -> Result<Rc<Class>, String> {
        let short_name = short_name_from_full_name(&full_name);

        for base in bases {
            if !base.is_trait() {
                return Err(format!(
                    "{} cannot be a base for {} because base classes must be traits",
                    base.full_name, full_name,
                ));
            }
            for (key, value) in &base.map {
                if !map.contains_key(key) {
                    map.insert(key.clone(), value.clone());
                }
            }
        }

        let fields = match kind {
            ClassKind::Trait | ClassKind::NativeClass => {
                if fields.is_some() {
                    return Err(format!("Traits and native classes cannot have fields"));
                } else {
                    vec![]
                }
            }
            ClassKind::UserDefinedClass
            | ClassKind::UserDefinedCaseClass
            | ClassKind::UserDefinedMutableClass => {
                if let Some(fields) = fields {
                    fields
                } else {
                    return Err(format!("User defined classes require a fields list"));
                }
            }
        };

        let has_static_call = static_map.contains_key(&Symbol::DUNDER_CALL);

        Ok(Class {
            kind,
            full_name,
            short_name,
            doc,
            fields: fields.clone(),
            map,
            static_map,
            fields_as_parameter_info: ParameterInfo::new(fields, vec![], None, None),
            has_static_call,
        }
        .into())
    }

    pub fn kind(&self) -> ClassKind {
        self.kind
    }

    pub fn is_trait(&self) -> bool {
        match self.kind {
            ClassKind::Trait => true,
            _ => false,
        }
    }

    pub fn full_name(&self) -> &RcStr {
        &self.full_name
    }

    pub fn short_name(&self) -> &RcStr {
        &self.short_name
    }

    pub fn doc(&self) -> &Option<RcStr> {
        &self.doc
    }

    pub fn instance_keys(&self) -> Vec<Symbol> {
        let mut keys: Vec<_> = self.map.keys().map(|key| *key).collect();
        keys.sort();
        keys
    }

    pub fn static_keys(&self) -> Vec<Symbol> {
        let mut keys: Vec<_> = self.static_map.keys().map(|key| *key).collect();
        keys.sort();
        keys
    }

    pub fn get_from_instance_map<K>(&self, name: &K) -> Option<&Value>
    where
        Symbol: std::borrow::Borrow<K>,
        K: ?Sized + Eq + std::hash::Hash,
    {
        self.map.get(name)
    }

    pub fn get_static<K>(&self, name: &K) -> Option<&Value>
    where
        Symbol: std::borrow::Borrow<K>,
        K: ?Sized + Eq + std::hash::Hash,
    {
        self.static_map.get(name)
    }

    pub(crate) fn has_static_call(&self) -> bool {
        self.has_static_call
    }

    pub fn instantiate(
        cls: &Rc<Class>,
        globals: &mut Globals,
        args: Vec<Value>,
        kwargs: Option<HashMap<Symbol, Value>>,
    ) -> EvalResult<Value> {
        match cls.kind {
            ClassKind::UserDefinedClass | ClassKind::UserDefinedCaseClass => {
                let (args, kwargs) = match cls.fields_as_parameter_info.translate(args, kwargs) {
                    Ok(pair) => pair,
                    Err(error) => return globals.set_exc_legacy(error.into()),
                };
                assert!(kwargs.is_none());
                let mut map = HashMap::new();
                for (key, arg) in cls.fields.iter().zip(args) {
                    map.insert(*key, arg);
                }
                Ok(UserObject {
                    cls: cls.clone(),
                    map,
                }
                .into())
            }
            ClassKind::UserDefinedMutableClass => {
                let (args, kwargs) = match cls.fields_as_parameter_info.translate(args, kwargs) {
                    Ok(pair) => pair,
                    Err(error) => return globals.set_exc_legacy(error.into()),
                };
                assert!(kwargs.is_none());
                let mut map = HashMap::new();
                for (key, arg) in cls.fields.iter().zip(args) {
                    map.insert(*key, RefCell::new(arg));
                }
                Ok(MutableUserObject {
                    cls: cls.clone(),
                    map,
                }
                .into())
            }
            _ => globals.set_exc_str("Only user defined classes may be instantiated"),
        }
    }
}

pub struct NativeIterator {
    f: Box<dyn FnMut(&mut Globals, Value) -> GeneratorResult>,
}
impl fmt::Debug for NativeIterator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<native-iterator>")
    }
}
impl NativeIterator {
    pub fn new<F>(f: F) -> NativeIterator
    where
        F: FnMut(&mut Globals, Value) -> GeneratorResult + 'static,
    {
        NativeIterator { f: Box::new(f) }
    }

    pub fn resume(&mut self, globals: &mut Globals, value: Value) -> GeneratorResult {
        (self.f)(globals, value)
    }

    pub fn next(&mut self, globals: &mut Globals) -> EvalResult<Option<Value>> {
        match self.resume(globals, Value::Nil) {
            GeneratorResult::Yield(value) => Ok(Some(value)),
            GeneratorResult::Done(_) => Ok(None),
            GeneratorResult::Error => Err(ErrorIndicator),
        }
    }
}

enum GeneratorStatus {
    NotStarted,
    InProgress,
    Done,
}
pub struct GeneratorObject {
    status: GeneratorStatus,
    frame: Frame,
    code: Rc<Code>,
}
impl fmt::Debug for GeneratorObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<generator-object {}>", self.code.short_name())
    }
}
impl GeneratorObject {
    pub fn next(&mut self, globals: &mut Globals) -> EvalResult<Option<Value>> {
        match self.resume(globals, Value::Nil) {
            GeneratorResult::Yield(value) => Ok(Some(value)),
            GeneratorResult::Done(_) => Ok(None),
            GeneratorResult::Error => Err(ErrorIndicator),
        }
    }
    pub fn resume(&mut self, globals: &mut Globals, value: Value) -> GeneratorResult {
        match &self.status {
            GeneratorStatus::NotStarted => {
                if value.is_nil() {
                    self.status = GeneratorStatus::InProgress;
                    self.code.start(globals, &mut self.frame)
                } else {
                    assert!(globals
                        .set_exc_legacy::<()>(EvalError::GeneratorStartedWithNonNilValue)
                        .is_err());
                    GeneratorResult::Error
                }
            }
            GeneratorStatus::InProgress => {
                match self.code.resume(globals, &mut self.frame, value) {
                    GeneratorResult::Done(value) => {
                        self.status = GeneratorStatus::Done;
                        GeneratorResult::Done(value)
                    }
                    result => result,
                }
            }
            GeneratorStatus::Done => {
                assert!(globals
                    .set_exc_legacy::<()>(EvalError::GeneratorResumeAfterDone)
                    .is_err());
                GeneratorResult::Error
            }
        }
    }
}

pub struct Module {
    name: RcStr,
    doc: Option<RcStr>,
    map: HMap<Symbol, Rc<RefCell<Value>>>,
}
impl Module {
    pub fn new(
        name: RcStr,
        doc: Option<RcStr>,
        map: HMap<Symbol, Rc<RefCell<Value>>>,
    ) -> Rc<Module> {
        Module { name, doc, map }.into()
    }

    pub fn name(&self) -> &RcStr {
        &self.name
    }

    pub fn doc(&self) -> &Option<RcStr> {
        &self.doc
    }

    /// Looks up the documentation associated with a specific Module member.
    /// If the __doc_XX field is available, that will be returned,
    /// otherwise, if the value itself is a function or class, the documentation
    /// associated with the function or class with be returned
    pub fn member_doc(&self, globals: &mut Globals, name: Symbol) -> EvalResult<Option<RcStr>> {
        let doc_name = Symbol::from(&format!("__doc_{}", name));
        if let Some(doc) = self.get(&doc_name) {
            let doc = crate::Eval::expect_string(globals, &doc)?.clone();
            Ok(Some(doc))
        } else {
            match self.get(&name) {
                Some(Value::Function(f)) => Ok(f.doc().clone()),
                Some(Value::Class(cls)) => Ok(cls.doc().clone()),
                _ => Ok(None),
            }
        }
    }

    pub fn map(&mut self) -> &HMap<Symbol, Rc<RefCell<Value>>> {
        &self.map
    }

    pub fn keys(&self) -> impl Iterator<Item = &Symbol> {
        self.map.keys()
    }

    pub fn map_clone(&self) -> HMap<Symbol, Value> {
        self.map
            .clone()
            .into_iter()
            .map(|(k, v)| (k, v.borrow().clone()))
            .collect()
    }

    pub fn get(&self, key: &Symbol) -> Option<Value> {
        self.map.get(key).map(|cell| cell.borrow().clone())
    }
}
impl fmt::Debug for Module {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<module {}>", self.name)
    }
}

pub struct HandleData {
    type_name: &'static str,
    cls: Rc<Class>,
    value: RefCell<Box<dyn Any>>,
}

impl fmt::Debug for HandleData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<handle {}>", self.type_name)
    }
}

impl HandleData {
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }
    pub fn cls(&self) -> &Rc<Class> {
        &self.cls
    }
}

pub struct Handle<T: Any>(Rc<HandleData>, PhantomData<T>);

impl<T: Any> Handle<T> {
    pub(crate) fn new(t: T, cls: Rc<Class>) -> Self {
        Self(
            Rc::new(HandleData {
                type_name: std::any::type_name::<T>(),
                cls,
                value: RefCell::new(Box::new(t)),
            }),
            PhantomData,
        )
    }
    pub fn type_name(&self) -> &'static str {
        self.0.type_name
    }
    pub fn cls(&self) -> &Rc<Class> {
        &self.0.cls
    }
    pub fn borrow(&self) -> Ref<T> {
        Ref::map(self.0.value.borrow(), |bx| bx.downcast_ref().unwrap())
    }
    pub fn borrow_mut(&self) -> RefMut<T> {
        RefMut::map(self.0.value.borrow_mut(), |bx| bx.downcast_mut().unwrap())
    }
    pub fn try_unwrap(self) -> std::result::Result<T, Self> {
        match Rc::try_unwrap(self.0) {
            Ok(data) => Ok(*data.value.into_inner().downcast().unwrap()),
            Err(ptr) => Err(Self(ptr, PhantomData)),
        }
    }
}

impl<T: Any + Clone> Handle<T> {
    pub fn unwrap_or_clone(self) -> T {
        match self.try_unwrap() {
            Ok(t) => t,
            Err(handle) => handle.borrow().clone(),
        }
    }
}

impl<T: Any> From<Handle<T>> for Value {
    fn from(handle: Handle<T>) -> Self {
        Self::Handle(handle.0)
    }
}

impl<T: Any> From<&Handle<T>> for Value {
    fn from(handle: &Handle<T>) -> Self {
        Self::Handle(handle.0.clone())
    }
}

/// Like Cow kind of, but instead of either Owned or Borrowed,
/// it's Owned or Handle
pub enum HCow<T: Any> {
    Owned(T),
    Handle(Handle<T>),
}

impl<T: Any> HCow<T> {
    pub fn with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        match self {
            Self::Owned(t) => f(t),
            Self::Handle(handle) => f(&handle.borrow()),
        }
    }

    pub fn with_mut<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        match self {
            Self::Owned(t) => f(t),
            Self::Handle(handle) => f(&mut handle.borrow_mut()),
        }
    }
}

impl<T: Any + Clone> HCow<T> {
    pub fn unwrap_or_clone(self) -> T {
        match self {
            Self::Owned(t) => t,
            Self::Handle(handle) => handle.borrow().clone(),
        }
    }
}

impl<T: Any> From<T> for HCow<T> {
    fn from(t: T) -> Self {
        Self::Owned(t)
    }
}

impl<T: Any> From<Handle<T>> for HCow<T> {
    fn from(t: Handle<T>) -> Self {
        Self::Handle(t)
    }
}
