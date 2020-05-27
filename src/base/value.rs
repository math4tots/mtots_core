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
use crate::SymbolRegistryHandle;
use crate::VMap;
use std::any::Any;
use std::cell::Ref;
use std::cell::RefCell;
use std::cell::RefMut;
use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum Value {
    Uninitialized,
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    Symbol(Symbol),
    String(RcStr),
    Bytes(Rc<Vec<u8>>),
    Path(RcPath),
    List(Rc<Vec<Value>>),       // [x, ..]
    Table(Rc<Table>),           // Table(k=v, ..)
    Map(Rc<VMap>),              // [k:v, ..]
    UserObject(Rc<UserObject>), // [k:v, ..]
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
    Opaque(Rc<Opaque>),

    // mutable values
    MutableString(Rc<RefCell<String>>),   // @".."
    MutableList(Rc<RefCell<Vec<Value>>>), // @[x, ..]
    MutableMap(Rc<RefCell<VMap>>),        // @[k:v, ..]

    // 'internal' values
    Cell(Rc<RefCell<Value>>), // for implementing closures
}

impl Value {
    pub fn is(&self, other: &Value) -> bool {
        fn ptr<T: ?Sized>(p: &Rc<T>) -> *const T {
            let p: &T = &*p;
            p as *const T
        }
        match (self, other) {
            (Value::Uninitialized, Value::Uninitialized) => true,
            (Value::Nil, Value::Nil) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Symbol(a), Value::Symbol(b)) => a == b,
            (Value::String(a), Value::String(b)) => a.as_ptr() == b.as_ptr(),
            (Value::Bytes(a), Value::Bytes(b)) => ptr(a) == ptr(b),
            (Value::Path(a), Value::Path(b)) => a.is_ptr_eq(b),
            (Value::List(a), Value::List(b)) => ptr(a) == ptr(b),
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
            (Value::Opaque(a), Value::Opaque(b)) => ptr(a) == ptr(b),
            (Value::MutableString(a), Value::MutableString(b)) => ptr(a) == ptr(b),
            (Value::MutableList(a), Value::MutableList(b)) => ptr(a) == ptr(b),
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
        if let Value::Int(x) = self {
            Some(*x)
        } else {
            None
        }
    }

    pub fn float(&self) -> Option<f64> {
        if let Value::Float(x) = self {
            Some(*x)
        } else {
            None
        }
    }

    pub fn floatlike(&self) -> Option<f64> {
        match self {
            Value::Int(x) => Some(*x as f64),
            Value::Float(x) => Some(*x),
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

    pub fn pathlike(&self) -> Option<RcPath> {
        match self {
            Value::String(x) => Some(Path::new(&**x).into()),
            Value::Path(x) => Some(x.clone()),
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

    pub fn kind(&self) -> ValueKind {
        match self {
            Value::Uninitialized => ValueKind::Uninitialized,
            Value::Nil => ValueKind::Nil,
            Value::Bool(..) => ValueKind::Bool,
            Value::Int(..) => ValueKind::Int,
            Value::Float(..) => ValueKind::Float,
            Value::Symbol(..) => ValueKind::Symbol,
            Value::String(..) => ValueKind::String,
            Value::Bytes(..) => ValueKind::Bytes,
            Value::Path(..) => ValueKind::Path,
            Value::List(..) => ValueKind::List,
            Value::Table(..) => ValueKind::Table,
            Value::Map(..) => ValueKind::Map,
            Value::UserObject(..) => ValueKind::UserObject,
            Value::Exception(..) => ValueKind::Exception,
            Value::NativeFunction(..) => ValueKind::NativeFunction,
            Value::NativeClosure(..) => ValueKind::NativeClosure,
            Value::Module(..) => ValueKind::Module,
            Value::Opaque(..) => ValueKind::Opaque,
            Value::Code(..) => ValueKind::Code,
            Value::Function(..) => ValueKind::Function,
            Value::Class(..) => ValueKind::Class,
            Value::ExceptionKind(..) => ValueKind::ExceptionKind,
            Value::NativeIterator(..) => ValueKind::NativeIterator,
            Value::GeneratorObject(..) => ValueKind::GeneratorObject,
            Value::MutableString(..) => ValueKind::MutableString,
            Value::MutableList(..) => ValueKind::MutableList,
            Value::MutableMap(..) => ValueKind::MutableMap,
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
            Value::Int(x) => write!(f, "{}", x),
            Value::Float(x) => write!(f, "{}", x),
            Value::Symbol(x) => write!(f, "{}", x),
            Value::String(s) => write!(f, "{}", s),
            Value::Bytes(s) => write!(f, "{:?}", s),
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
            Value::Opaque(opq) => write!(f, "{:?}", opq),
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
    Float,
    Symbol,
    String,
    Bytes,
    Path,
    List,
    Table,
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
    Opaque,
    MutableString,
    MutableList,
    MutableMap,
    Cell,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum ConstValue {
    Uninitialized,
    Nil,
    Bool(bool),
    Int(i64),
    Float(u64),
    Symbol(Symbol),
    String(RcStr),
    Path(RcPath),
}

impl<T: Into<ConstValue>> From<T> for Value {
    fn from(cv: T) -> Value {
        let cv = cv.into();
        match cv {
            ConstValue::Uninitialized => Value::Uninitialized,
            ConstValue::Nil => Value::Nil,
            ConstValue::Bool(x) => Value::Bool(x),
            ConstValue::Int(x) => Value::Int(x),
            ConstValue::Float(x) => Value::Float(f64::from_bits(x)),
            ConstValue::Symbol(x) => Value::Symbol(x),
            ConstValue::String(x) => Value::String(x),
            ConstValue::Path(x) => Value::Path(x),
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
        ConstValue::Int(x)
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

    pub fn get_class(&self) -> &Rc<Class> {
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
    pub fn new(
        required: Vec<Symbol>,
        optional: Vec<(Symbol, Value)>,
        variadic: Option<Symbol>,
        keywords: Option<Symbol>,
    ) -> ParameterInfo {
        let kind = if required.is_empty()
            && optional.is_empty()
            && variadic.is_some()
            && keywords.is_some()
        {
            ParameterInfoKind::PassThrough
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
    /// Like new, but accepts &str values
    pub fn snew(
        sr: &SymbolRegistryHandle,
        req: &[&str],
        opt: &[(&str, Value)],
        var: Option<&str>,
        key: Option<&str>,
    ) -> ParameterInfo {
        let req = req.iter().map(|s| sr.intern_str(s)).collect();
        let opt = opt
            .iter()
            .map(|(s, v)| (sr.intern_str(s), v.clone()))
            .collect();
        let var = var.map(|s| sr.intern_str(s));
        let key = key.map(|s| sr.intern_str(s));
        Self::new(req, opt, var, key)
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

type FunctionResult = EvalResult<Value>;
pub type NativeFunctionBody =
    fn(&mut Globals, args: Vec<Value>, kwargs: Option<HashMap<Symbol, Value>>) -> FunctionResult;
pub struct NativeFunction {
    name: RcStr,
    parameter_info: ParameterInfo,
    body: NativeFunctionBody,
}
impl From<NativeFunction> for Value {
    fn from(f: NativeFunction) -> Value {
        Value::NativeFunction(f.into())
    }
}
impl fmt::Debug for NativeFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<native function {}>", self.name())
    }
}
impl NativeFunction {
    pub fn new(
        name: RcStr,
        parameter_info: ParameterInfo,
        body: NativeFunctionBody,
    ) -> NativeFunction {
        NativeFunction {
            name,
            parameter_info,
            body,
        }
    }
    pub fn snew(
        sr: &SymbolRegistryHandle,
        name: &str,
        parameter_info: (&[&str], &[(&str, Value)], Option<&str>, Option<&str>),
        body: NativeFunctionBody,
    ) -> NativeFunction {
        Self::new(
            name.into(),
            ParameterInfo::snew(
                sr,
                parameter_info.0,
                parameter_info.1,
                parameter_info.2,
                parameter_info.3,
            ),
            body,
        )
    }
    pub fn simple0(
        symbol_registry: &SymbolRegistryHandle,
        name: &str,
        params: &[&str],
        body: NativeFunctionBody,
    ) -> NativeFunction {
        let params = params
            .iter()
            .map(|s| symbol_registry.intern_str(s))
            .collect();
        Self::new(
            name.into(),
            ParameterInfo::new(params, vec![], None, None),
            body,
        )
    }
    pub fn name(&self) -> &RcStr {
        &self.name
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
    body: NativeClosureBody,
}
impl From<NativeClosure> for Value {
    fn from(f: NativeClosure) -> Value {
        Value::NativeClosure(f.into())
    }
}
impl fmt::Debug for NativeClosure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<native closure {}>", self.name())
    }
}
impl NativeClosure {
    pub fn new<F>(name: RcStr, parameter_info: ParameterInfo, body: F) -> NativeClosure
    where
        F: Fn(&mut Globals, Vec<Value>, Option<HashMap<Symbol, Value>>) -> FunctionResult + 'static,
    {
        NativeClosure {
            name,
            parameter_info,
            body: Box::new(body),
        }
    }
    pub fn name(&self) -> &RcStr {
        &self.name
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
}
pub struct Class {
    kind: ClassKind,
    full_name: RcStr,
    short_name: RcStr,
    fields: Vec<Symbol>,
    map: HashMap<Symbol, Value>,
    static_map: HashMap<Symbol, Value>,
    fields_as_parameter_info: ParameterInfo,
    has_static_call: bool,
}
impl fmt::Debug for Class {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<class {}>", self.short_name)
    }
}
impl Class {
    pub fn new(
        globals: &mut Globals,
        kind: ClassKind,
        full_name: RcStr,
        bases: Vec<Rc<Class>>,
        fields: Option<Vec<Symbol>>,
        map: HashMap<Symbol, Value>,
        static_map: HashMap<Symbol, Value>,
    ) -> EvalResult<Rc<Class>> {
        match Self::new00(kind, full_name, bases, fields, map, static_map) {
            Ok(cls) => Ok(cls),
            Err(error) => globals.set_exc_other(error.into()),
        }
    }
    /// version of new for creating builtin classes
    pub(crate) fn new0(
        kind: ClassKind,
        full_name: RcStr,
        bases: Vec<Rc<Class>>,
        map: HashMap<Symbol, Value>,
        static_map: HashMap<Symbol, Value>,
    ) -> Rc<Class> {
        Self::new00(kind, full_name, bases, None, map, static_map).unwrap()
    }
    fn new00(
        kind: ClassKind,
        full_name: RcStr,
        bases: Vec<Rc<Class>>,
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
            ClassKind::UserDefinedClass => {
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
    ) -> EvalResult<UserObject> {
        if let ClassKind::UserDefinedClass = cls.kind {
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
            })
        } else {
            return globals.set_exc_str("Only user defined classes may be instantiated");
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
    map: HMap<Symbol, Rc<RefCell<Value>>>,
}
impl Module {
    pub fn new(name: RcStr, map: HMap<Symbol, Rc<RefCell<Value>>>) -> Rc<Module> {
        Module { name, map }.into()
    }

    pub fn name(&self) -> &RcStr {
        &self.name
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

pub struct Opaque {
    type_name: &'static str,
    ptr: RefCell<Option<Box<dyn Any>>>,
}
impl From<Opaque> for Value {
    fn from(opq: Opaque) -> Value {
        Value::Opaque(opq.into())
    }
}
impl From<Rc<Opaque>> for Value {
    fn from(opq: Rc<Opaque>) -> Value {
        Value::Opaque(opq)
    }
}
impl Opaque {
    pub fn new<T: 'static>(t: T) -> Opaque {
        let type_name = std::any::type_name::<T>();
        Opaque {
            type_name,
            ptr: RefCell::new(Some(Box::new(t))),
        }
    }
    pub fn moved(&self) -> bool {
        self.ptr.borrow().is_none()
    }
    pub fn type_name(&self) -> &'static str {
        if self.moved() {
            "<moved>"
        } else {
            self.type_name
        }
    }
    fn can_downcast_to<T: 'static>(&self) -> bool {
        let ptr = self.ptr.borrow();
        if let Some(ptr) = ptr.as_ref() {
            let opt: Option<&T> = ptr.downcast_ref();
            opt.is_some()
        } else {
            false
        }
    }
    pub fn borrow<'a, T: 'static>(&'a self) -> Option<Ref<'a, T>> {
        if self.can_downcast_to::<T>() {
            Some(Ref::map(self.ptr.borrow(), |ptr| {
                ptr.as_ref().unwrap().downcast_ref().unwrap()
            }))
        } else {
            None
        }
    }
    pub fn borrow_mut<'a, T: 'static>(&'a self) -> Option<RefMut<'a, T>> {
        if self.can_downcast_to::<T>() {
            Some(RefMut::map(self.ptr.borrow_mut(), |ptr| {
                ptr.as_mut().unwrap().downcast_mut().unwrap()
            }))
        } else {
            None
        }
    }
    pub fn move_<T: 'static>(&self) -> Option<T> {
        if self.can_downcast_to::<T>() {
            let ptr = self.ptr.replace(None).unwrap();
            Some(*ptr.downcast().unwrap())
        } else {
            None
        }
    }
}
impl fmt::Debug for Opaque {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<opaque {}", self.type_name)?;
        if self.moved() {
            write!(f, " (moved)")?;
        }
        write!(f, ">")
    }
}