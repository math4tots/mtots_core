/// Operations on Value objects
use crate::divmod;
use crate::HCow;
use crate::ArgumentError;
use crate::Class;
use crate::ClassKind;
use crate::Code;
use crate::CompileErrorKind;
use crate::ErrorIndicator;
use crate::Exception;
use crate::ExceptionKind;
use crate::FailableEq;
use crate::FailableHash;
use crate::Function;
use crate::GMap;
use crate::GeneratorResult;
use crate::Globals;
use crate::Handle;
use crate::LexErrorKind;
use crate::Module;
use crate::NativeFunction;
use crate::NativeIterator;
use crate::Operation;
use crate::ParameterInfo;
use crate::ParseErrorKind;
use crate::RcPath;
use crate::RcStr;
use crate::Symbol;
use crate::Table;
use crate::UnorderedHasher;
use crate::Value;
use crate::ValueKind;
use std::any::Any;
use std::cell::Ref;
use std::cell::RefCell;
use std::cell::RefMut;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Debug)]
pub enum EvalError {
    UninitializedValue,
    ArgumentError(ArgumentError),
    NotCallable(Value),
    NoSuchMethod(RcStr, Rc<Class>),
    MakeFunctionInvalidCellsValue,
    CompileError(CompileErrorKind),
    LexError(LexErrorKind),
    ParseError(ParseErrorKind),
    IOError(std::io::Error),
    NotUnicode(std::ffi::OsString),
    ModuleNotFound,
    YieldOutsideGenerator,
    GeneratorStartedWithNonNilValue,
    GeneratorResumeAfterDone,
    UnpackSize { expected: usize, but_got: usize },
    ExpectedIterable(ValueKind),
    ExpectedIterator(ValueKind),
    NoSuchAttribute(Symbol, RcStr),
    CouldNotAssignAttribute(Symbol),
    OperationNotSupportedForKinds(Operation, Vec<ValueKind>),
}

impl EvalError {
    pub fn tag(&self) -> &'static str {
        match self {
            EvalError::UninitializedValue => "UninitializedValue",
            EvalError::ArgumentError(_) => "ArgumentError",
            EvalError::NotCallable(_) => "NotCallable",
            EvalError::NoSuchMethod(_, _) => "NoSuchMethod",
            EvalError::MakeFunctionInvalidCellsValue => "MakeFunctionInvalidCellsValue",
            EvalError::CompileError(_) => "CompileError",
            EvalError::LexError(_) => "LexError",
            EvalError::ParseError(_) => "ParseError",
            EvalError::IOError(_) => "IOError",
            EvalError::NotUnicode(_) => "NotUnicode",
            EvalError::ModuleNotFound => "ModuleNotFound",
            EvalError::YieldOutsideGenerator => "YieldOutsideGenerator",
            EvalError::GeneratorStartedWithNonNilValue => "GeneratorStartedWithNonNilValue",
            EvalError::GeneratorResumeAfterDone => "GeneratorResumeAfterDone",
            EvalError::UnpackSize { .. } => "UnpackSize",
            EvalError::ExpectedIterable(_) => "ExpectedIterable",
            EvalError::ExpectedIterator(_) => "ExpectedIterator",
            EvalError::NoSuchAttribute(_, _) => "NoSuchAttribute",
            EvalError::CouldNotAssignAttribute(_) => "CouldNotAssignAttribute",
            EvalError::OperationNotSupportedForKinds(_, _) => "OperationNotSupportedForKinds",
        }
    }
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.tag())?;
        match self {
            EvalError::UninitializedValue => write!(f, "Variable used before set"),
            EvalError::ArgumentError(err) => write!(f, "{}", err),
            EvalError::NotCallable(c) => write!(f, "{:?} is not callable", c),
            EvalError::NoSuchMethod(name, cls) => write!(
                f,
                "Method {:?} not found for class {}",
                name,
                cls.full_name()
            ),
            EvalError::MakeFunctionInvalidCellsValue => write!(f, ""),
            EvalError::CompileError(kind) => write!(f, "{}", kind),
            EvalError::LexError(kind) => write!(f, "{:?}", kind),
            EvalError::ParseError(kind) => write!(f, "{:?}", kind),
            EvalError::IOError(err) => write!(f, "{:?}", err),
            EvalError::NotUnicode(osstr) => write!(f, "{:?} is not a valid unicode string", osstr),
            EvalError::ModuleNotFound => write!(f, ""),
            EvalError::YieldOutsideGenerator => {
                write!(f, "Yield may not be used outside a generator function")
            }
            EvalError::GeneratorStartedWithNonNilValue => {
                write!(f, "The first resume on a generator must be with nil")
            }
            EvalError::GeneratorResumeAfterDone => {
                write!(f, "Generator was resumed after it was exhausted")
            }
            EvalError::UnpackSize { expected, but_got } => {
                write!(f, "Expected {} values but got {}", expected, but_got)
            }
            EvalError::ExpectedIterable(kind) => write!(f, "Expected iterable but got {:?}", kind),
            EvalError::ExpectedIterator(kind) => write!(f, "Expected iterator but got {:?}", kind),
            EvalError::NoSuchAttribute(name, clsname) => write!(
                f,
                "Attribute {:?} not found for instance of type {:?}",
                name.str(),
                clsname
            ),
            EvalError::CouldNotAssignAttribute(name) => write!(f, "{:?}", name.str()),
            EvalError::OperationNotSupportedForKinds(op, kinds) => {
                write!(f, "{:?}", op)?;
                for kind in kinds {
                    write!(f, ", {:?}", kind)?;
                }
                Ok(())
            }
        }
    }
}

impl From<ArgumentError> for EvalError {
    fn from(err: ArgumentError) -> EvalError {
        EvalError::ArgumentError(err)
    }
}
impl From<std::io::Error> for EvalError {
    fn from(err: std::io::Error) -> EvalError {
        EvalError::IOError(err)
    }
}

pub type EvalResult<T> = Result<T, ErrorIndicator>;
pub struct Eval {}
impl Eval {
    pub fn classof<'a>(globals: &'a mut Globals, value: &'a Value) -> EvalResult<&'a Rc<Class>> {
        Ok(match value {
            Value::Uninitialized => return globals.set_exc_legacy(EvalError::UninitializedValue),
            Value::Nil => &globals.builtin_classes().Nil,
            Value::Bool(_) => &globals.builtin_classes().Bool,
            Value::Int(_) => &globals.builtin_classes().Int,
            Value::Float(_) => &globals.builtin_classes().Float,
            Value::Symbol(_) => &globals.builtin_classes().Symbol,
            Value::String(_) => &globals.builtin_classes().String,
            Value::Bytes(_) => &globals.builtin_classes().Bytes,
            Value::Path(_) => &globals.builtin_classes().Path,
            Value::List(_) => &globals.builtin_classes().List,
            Value::Table(_) => &globals.builtin_classes().Table,
            Value::Set(_) => &globals.builtin_classes().Set,
            Value::Map(_) => &globals.builtin_classes().Map,
            Value::UserObject(obj) => obj.cls(),
            Value::Exception(_) => &globals.builtin_classes().Exception,
            Value::NativeFunction(_) => &globals.builtin_classes().NativeFunction,
            Value::NativeClosure(_) => &globals.builtin_classes().NativeClosure,
            Value::Code(_) => &globals.builtin_classes().Code,
            Value::Function(_) => &globals.builtin_classes().Function,
            Value::Class(_) => &globals.builtin_classes().Class,
            Value::ExceptionKind(_) => &globals.builtin_classes().ExceptionKind,
            Value::NativeIterator(_) => &globals.builtin_classes().NativeIterator,
            Value::GeneratorObject(_) => &globals.builtin_classes().GeneratorObject,
            Value::Module(_) => &globals.builtin_classes().Module,
            Value::Handle(handle) => handle.cls(),
            Value::MutableString(_) => &globals.builtin_classes().MutableString,
            Value::MutableBytes(_) => &globals.builtin_classes().MutableBytes,
            Value::MutableList(_) => &globals.builtin_classes().MutableList,
            Value::MutableSet(_) => &globals.builtin_classes().MutableSet,
            Value::MutableMap(_) => &globals.builtin_classes().MutableMap,
            Value::MutableUserObject(obj) => obj.cls(),
            Value::Cell(_) => &globals.builtin_classes().Cell,
        })
    }

    pub fn expect_int(globals: &mut Globals, value: &Value) -> EvalResult<i64> {
        if let Some(int) = value.int() {
            Ok(int)
        } else {
            globals.set_kind_error(ValueKind::Int, value.kind())
        }
    }

    pub fn expect_uint(globals: &mut Globals, value: &Value) -> EvalResult<u64> {
        if let Value::Int(i) = value {
            if *i < 0 {
                globals.set_exc_str(&format!("Expected non-negative int, but got {}", i))
            } else {
                Ok(*i as u64)
            }
        } else {
            globals.set_kind_error(ValueKind::Int, value.kind())
        }
    }

    pub fn expect_i32(globals: &mut Globals, value: &Value) -> EvalResult<i32> {
        let x = Self::expect_int(globals, value)?;
        Self::check_i32(globals, x)
    }

    pub fn expect_i16(globals: &mut Globals, value: &Value) -> EvalResult<i16> {
        let x = Self::expect_int(globals, value)?;
        Self::check_i16(globals, x)
    }

    pub fn expect_i8(globals: &mut Globals, value: &Value) -> EvalResult<i8> {
        let x = Self::expect_int(globals, value)?;
        Self::check_i8(globals, x)
    }

    pub fn expect_u32(globals: &mut Globals, value: &Value) -> EvalResult<u32> {
        let x = Self::expect_int(globals, value)?;
        Self::check_u32(globals, x)
    }

    pub fn expect_u16(globals: &mut Globals, value: &Value) -> EvalResult<u16> {
        let x = Self::expect_int(globals, value)?;
        Self::check_u16(globals, x)
    }

    pub fn expect_u8(globals: &mut Globals, value: &Value) -> EvalResult<u8> {
        let x = Self::expect_int(globals, value)?;
        Self::check_u8(globals, x)
    }

    pub fn check_u8(globals: &mut Globals, x: i64) -> EvalResult<u8> {
        if x < 0 || x > u8::MAX as i64 {
            globals.set_exc_str(&format!("Expected u8 but got {}", x))
        } else {
            Ok(x as u8)
        }
    }

    pub fn check_u16(globals: &mut Globals, x: i64) -> EvalResult<u16> {
        if x < 0 || x > u16::MAX as i64 {
            globals.set_exc_str(&format!("Expected u16 but got {}", x))
        } else {
            Ok(x as u16)
        }
    }

    pub fn check_u32(globals: &mut Globals, x: i64) -> EvalResult<u32> {
        if x < 0 || x > u32::MAX as i64 {
            globals.set_exc_str(&format!("Expected u32 but got {}", x))
        } else {
            Ok(x as u32)
        }
    }

    pub fn check_i8(globals: &mut Globals, x: i64) -> EvalResult<i8> {
        if x < i8::MIN as i64 || x > i8::MAX as i64 {
            globals.set_exc_str(&format!("Expected i8 but got {}", x))
        } else {
            Ok(x as i8)
        }
    }

    pub fn check_i16(globals: &mut Globals, x: i64) -> EvalResult<i16> {
        if x < i16::MIN as i64 || x > i16::MAX as i64 {
            globals.set_exc_str(&format!("Expected i16 but got {}", x))
        } else {
            Ok(x as i16)
        }
    }

    pub fn check_i32(globals: &mut Globals, x: i64) -> EvalResult<i32> {
        if x < i32::MIN as i64 || x > i32::MAX as i64 {
            globals.set_exc_str(&format!("Expected i32 but got {}", x))
        } else {
            Ok(x as i32)
        }
    }

    pub fn expect_usize(globals: &mut Globals, value: &Value) -> EvalResult<usize> {
        if let Value::Int(i) = value {
            if *i < 0 {
                globals.set_exc_str(&format!("Expected non-negative int, but got {}", i))
            } else {
                Ok(*i as usize)
            }
        } else {
            globals.set_kind_error(ValueKind::Int, value.kind())
        }
    }

    /// Like expect_index, but if the index is too small, it returns 0,
    /// and if the index is too big, will just return len
    pub fn expect_index_permissive(
        globals: &mut Globals,
        value: &Value,
        len: usize,
    ) -> EvalResult<usize> {
        let mut i = Self::expect_int(globals, value)?;
        if i < 0 {
            i += len as i64;
        }
        if i < 0 {
            Ok(0)
        } else if i > len as i64 {
            Ok(len)
        } else {
            Ok(i as usize)
        }
    }

    pub fn expect_range_indices(
        globals: &mut Globals,
        start: &Value,
        end: &Value,
        len: usize,
    ) -> EvalResult<(usize, usize)> {
        let start = if let Value::Nil = start {
            0
        } else {
            Eval::expect_index_permissive(globals, start, len)?
        };
        let end = if let Value::Nil = end {
            len
        } else {
            Eval::expect_index_permissive(globals, end, len)?
        };
        Ok((start, end))
    }

    pub fn expect_str_slice<'a>(
        globals: &mut Globals,
        string: &'a Value,
        start: &Value,
        end: &Value,
    ) -> EvalResult<(RcStr, usize, usize)> {
        let string = Eval::expect_string(globals, string)?;
        let len = string.charlen();
        let (start, end) = Eval::expect_range_indices(globals, start, end, len)?;
        Ok((string.charslice(start, end), start, end))
    }

    /// Like expect_index, but returns an Option
    pub fn try_index(value: &Value, len: usize) -> Option<usize> {
        match value {
            Value::Int(i) => {
                let mut i = *i;
                if i < 0 {
                    i += len as i64;
                }
                if i < 0 || i >= (len as i64) {
                    None
                } else {
                    Some(i as usize)
                }
            }
            _ => None,
        }
    }

    pub fn expect_index(globals: &mut Globals, value: &Value, len: usize) -> EvalResult<usize> {
        if let Some(i) = Self::try_index(value, len) {
            Ok(i)
        } else {
            // check if it's an int
            Self::expect_int(globals, value)?;
            // if it is, then try_index must have failed because it is out of bounds
            globals.set_exc_str("Index out of bounds")
        }
    }

    pub fn expect_float(globals: &mut Globals, value: &Value) -> EvalResult<f64> {
        if let Some(float) = value.float() {
            Ok(float)
        } else {
            globals.set_kind_error(ValueKind::Float, value.kind())
        }
    }

    pub fn expect_floatlike(globals: &mut Globals, value: &Value) -> EvalResult<f64> {
        if let Some(float) = value.floatlike() {
            Ok(float)
        } else {
            globals.set_kind_error(ValueKind::Float, value.kind())
        }
    }

    pub fn expect_symbol(globals: &mut Globals, value: &Value) -> EvalResult<Symbol> {
        if let Some(float) = value.symbol() {
            Ok(float)
        } else {
            globals.set_kind_error(ValueKind::Symbol, value.kind())
        }
    }

    pub fn expect_symbollike(globals: &mut Globals, value: &Value) -> EvalResult<Symbol> {
        match value {
            Value::Symbol(s) => Ok(*s),
            Value::String(s) => Ok(Symbol::from(s)),
            _ => globals.set_kind_error(ValueKind::Symbol, value.kind()),
        }
    }

    pub fn expect_string<'a>(globals: &mut Globals, value: &'a Value) -> EvalResult<&'a RcStr> {
        if let Some(string) = value.string() {
            Ok(string)
        } else {
            globals.set_kind_error(ValueKind::String, value.kind())
        }
    }
    pub fn move_or_clone_string(globals: &mut Globals, value: Value) -> EvalResult<String> {
        if let Value::String(rc) = value {
            Ok(RcStr::unwrap_or_clone(rc))
        } else {
            globals.set_kind_error(ValueKind::Table, value.kind())
        }
    }

    pub fn expect_bytes<'a>(globals: &mut Globals, value: &'a Value) -> EvalResult<&'a Vec<u8>> {
        if let Value::Bytes(bytes) = value {
            Ok(bytes)
        } else {
            globals.set_kind_error(ValueKind::Bytes, value.kind())
        }
    }

    pub fn expect_bytes_from_pattern(globals: &mut Globals, value: &Value) -> EvalResult<Vec<u8>> {
        let mut bytes = Vec::new();
        Self::add_bytes(globals, &mut bytes, value)?;
        Ok(bytes)
    }

    fn add_bytes(globals: &mut Globals, bytes: &mut Vec<u8>, value: &Value) -> EvalResult<()> {
        match value {
            Value::Int(i) => {
                let i = Self::check_u8(globals, *i)?;
                bytes.push(i);
                Ok(())
            }
            Value::String(s) => {
                bytes.extend(s.as_bytes());
                Ok(())
            }
            Value::Bytes(bb) => {
                bytes.extend(bb.as_slice());
                Ok(())
            }
            Value::List(list) => {
                for subval in list.iter() {
                    Self::add_bytes(globals, bytes, subval)?;
                }
                Ok(())
            }
            Value::MutableBytes(bb) => {
                bytes.extend(bb.borrow().as_slice());
                Ok(())
            }
            _ => globals.set_kind_error(ValueKind::Bytes, value.kind()),
        }
    }

    pub fn expect_path<'a>(globals: &mut Globals, value: &'a Value) -> EvalResult<&'a RcPath> {
        if let Some(path) = value.path() {
            Ok(path)
        } else {
            globals.set_kind_error(ValueKind::Path, value.kind())
        }
    }
    pub fn move_or_clone_path(globals: &mut Globals, value: Value) -> EvalResult<PathBuf> {
        if let Value::Path(rc) = value {
            Ok(RcPath::unwrap_or_clone(rc))
        } else {
            globals.set_kind_error(ValueKind::Table, value.kind())
        }
    }
    pub fn expect_pathlike<'a>(globals: &mut Globals, value: &'a Value) -> EvalResult<&'a Path> {
        if let Some(pathlike) = value.pathlike() {
            Ok(pathlike)
        } else {
            globals.set_kind_error(ValueKind::Path, value.kind())
        }
    }
    pub fn expect_pathlike_rc(globals: &mut Globals, value: &Value) -> EvalResult<RcPath> {
        if let Value::Path(rc) = value {
            Ok(rc.clone())
        } else {
            Ok(Self::expect_pathlike(globals, value)?.into())
        }
    }

    pub fn expect_osstr<'a>(
        globals: &mut Globals,
        value: &'a Value,
    ) -> EvalResult<&'a std::ffi::OsStr> {
        match value {
            Value::Symbol(s) => Ok(s.str().as_ref()),
            Value::String(s) => Ok(s.str().as_ref()),
            Value::Path(p) => Ok(p.path().as_ref()),
            _ => globals.set_kind_error(ValueKind::String, value.kind()),
        }
    }

    pub fn expect_list<'a>(globals: &mut Globals, value: &'a Value) -> EvalResult<&'a Vec<Value>> {
        if let Some(list) = value.list() {
            Ok(list)
        } else {
            globals.set_kind_error(ValueKind::List, value.kind())
        }
    }

    /// Expects the given value to be the only holder of a list --
    /// and moves the vector out
    pub fn try_move_list(globals: &mut Globals, value: Value) -> EvalResult<Option<Vec<Value>>> {
        if let Value::List(rc) = value {
            match Rc::try_unwrap(rc) {
                Ok(list) => Ok(Some(list)),
                Err(_) => Ok(None),
            }
        } else {
            globals.set_kind_error(ValueKind::List, value.kind())
        }
    }

    pub fn move_list(globals: &mut Globals, value: Value) -> EvalResult<Vec<Value>> {
        match Self::try_move_list(globals, value)? {
            Some(list) => Ok(list),
            None => globals.set_exc_str("Expected unique reference to List instance"),
        }
    }

    pub fn move_or_clone_list(globals: &mut Globals, value: Value) -> EvalResult<Vec<Value>> {
        if let Value::List(rc) = value {
            match Rc::try_unwrap(rc) {
                Ok(list) => Ok(list),
                Err(list_rc) => Ok((*list_rc).clone()),
            }
        } else {
            globals.set_kind_error(ValueKind::List, value.kind())
        }
    }

    pub fn expect_table<'a>(globals: &mut Globals, value: &'a Value) -> EvalResult<&'a Table> {
        if let Some(table) = value.table() {
            Ok(table)
        } else {
            globals.set_kind_error(ValueKind::Table, value.kind())
        }
    }
    pub fn move_or_clone_table(globals: &mut Globals, value: Value) -> EvalResult<Table> {
        if let Value::Table(rc) = value {
            match Rc::try_unwrap(rc) {
                Ok(table) => Ok(table),
                Err(rc) => Ok((*rc).clone()),
            }
        } else {
            globals.set_kind_error(ValueKind::Table, value.kind())
        }
    }
    pub fn try_move_table(globals: &mut Globals, value: Value) -> EvalResult<Option<Table>> {
        if let Value::Table(rc) = value {
            match Rc::try_unwrap(rc) {
                Ok(table) => Ok(Some(table)),
                Err(_) => Ok(None),
            }
        } else {
            globals.set_kind_error(ValueKind::Table, value.kind())
        }
    }
    pub fn move_table(globals: &mut Globals, value: Value) -> EvalResult<Table> {
        match Self::try_move_table(globals, value)? {
            Some(table) => Ok(table),
            None => globals.set_exc_str("Expected unique reference to Table instance"),
        }
    }

    pub fn expect_set<'a>(globals: &mut Globals, value: &'a Value) -> EvalResult<&'a VSet> {
        if let Some(set) = value.set() {
            Ok(set)
        } else {
            globals.set_kind_error(ValueKind::Set, value.kind())
        }
    }

    pub fn expect_map<'a>(globals: &mut Globals, value: &'a Value) -> EvalResult<&'a VMap> {
        if let Some(map) = value.map() {
            Ok(map)
        } else {
            globals.set_kind_error(ValueKind::Map, value.kind())
        }
    }

    pub fn move_exc(globals: &mut Globals, value: Value) -> EvalResult<Exception> {
        if let Value::Exception(excptr) = value {
            match Rc::try_unwrap(excptr) {
                Ok(exc) => Ok(exc),
                Err(_) => globals.set_exc_str("Expected unique reference to Exception instance"),
            }
        } else {
            globals.set_kind_error(ValueKind::Exception, value.kind())
        }
    }

    pub fn expect_module<'a>(
        globals: &mut Globals,
        value: &'a Value,
    ) -> EvalResult<&'a Rc<Module>> {
        if let Some(module) = value.module() {
            Ok(module)
        } else {
            globals.set_kind_error(ValueKind::Module, value.kind())
        }
    }

    pub fn handle_borrow<'a, T: Any>(
        globals: &mut Globals,
        value: &'a Value,
    ) -> EvalResult<Ref<'a, T>> {
        if let Some(rf) = value.handle_borrow() {
            Ok(rf)
        } else {
            globals.set_kind_error(ValueKind::Handle(std::any::type_name::<T>()), value.kind())
        }
    }

    pub fn handle_borrow_mut<'a, T: Any>(
        globals: &mut Globals,
        value: &'a Value,
    ) -> EvalResult<RefMut<'a, T>> {
        if let Some(rf) = value.handle_borrow_mut() {
            Ok(rf)
        } else {
            globals.set_kind_error(ValueKind::Handle(std::any::type_name::<T>()), value.kind())
        }
    }

    pub fn into_handle<T: Any>(globals: &mut Globals, hcow: HCow<T>) -> EvalResult<Handle<T>> {
        match hcow {
            HCow::Handle(handle) => Ok(handle),
            HCow::Owned(t) => globals.new_handle(t),
        }
    }

    pub fn hcow_to_owned<T: Any>(globals: &mut Globals, hcow: HCow<T>) -> EvalResult<T> {
        match hcow {
            HCow::Handle(handle) => Self::unwrap_typed_handle(globals, handle),
            HCow::Owned(t) => Ok(t),
        }
    }

    pub fn expect_handle<T: Any>(globals: &mut Globals, value: &Value) -> EvalResult<Handle<T>> {
        if let Some(handle) = value.handle() {
            Ok(handle)
        } else {
            globals.set_kind_error(ValueKind::Handle(std::any::type_name::<T>()), value.kind())
        }
    }

    pub fn unwrap_typed_handle<T: Any>(globals: &mut Globals, handle: Handle<T>) -> EvalResult<T> {
        match handle.try_unwrap() {
            Ok(t) => Ok(t),
            Err(_) => globals.set_exc_str(&format!(
                "Could not unwrap handle to {}; more references still exist",
                std::any::type_name::<T>()
            )),
        }
    }

    pub fn unwrap_handle<T: Any>(globals: &mut Globals, value: Value) -> EvalResult<T> {
        match value.into_handle() {
            Ok(handle) => Self::unwrap_typed_handle(globals, handle),
            Err(value) => {
                globals.set_kind_error(ValueKind::Handle(std::any::type_name::<T>()), value.kind())
            }
        }
    }

    pub fn expect_mutable_string<'a>(
        globals: &mut Globals,
        value: &'a Value,
    ) -> EvalResult<&'a Rc<RefCell<String>>> {
        if let Some(string) = value.mutable_string() {
            Ok(string)
        } else {
            globals.set_kind_error(ValueKind::String, value.kind())
        }
    }

    pub fn expect_mutable_bytes<'a>(
        globals: &mut Globals,
        value: &'a Value,
    ) -> EvalResult<&'a Rc<RefCell<Vec<u8>>>> {
        if let Value::MutableBytes(bytes) = value {
            Ok(bytes)
        } else {
            globals.set_kind_error(ValueKind::MutableBytes, value.kind())
        }
    }

    pub fn expect_mutable_list<'a>(
        globals: &mut Globals,
        value: &'a Value,
    ) -> EvalResult<&'a Rc<RefCell<Vec<Value>>>> {
        if let Some(list) = value.mutable_list() {
            Ok(list)
        } else {
            globals.set_kind_error(ValueKind::MutableList, value.kind())
        }
    }

    pub fn expect_mutable_set<'a>(
        globals: &mut Globals,
        value: &'a Value,
    ) -> EvalResult<&'a Rc<RefCell<VSet>>> {
        if let Value::MutableSet(set) = &value {
            Ok(set)
        } else {
            globals.set_kind_error(ValueKind::MutableSet, value.kind())
        }
    }

    pub fn expect_mutable_map<'a>(
        globals: &mut Globals,
        value: &'a Value,
    ) -> EvalResult<&'a Rc<RefCell<VMap>>> {
        if let Value::MutableMap(map) = &value {
            Ok(map)
        } else {
            globals.set_kind_error(ValueKind::MutableMap, value.kind())
        }
    }

    pub fn expect_class<'a>(globals: &mut Globals, value: &'a Value) -> EvalResult<&'a Rc<Class>> {
        if let Value::Class(cls) = value {
            Ok(cls)
        } else {
            globals.set_kind_error(ValueKind::Class, value.kind())
        }
    }

    pub fn expect_native_func<'a>(
        globals: &mut Globals,
        value: &'a Value,
    ) -> EvalResult<&'a Rc<NativeFunction>> {
        if let Value::NativeFunction(func) = value {
            Ok(func)
        } else {
            globals.set_kind_error(ValueKind::NativeFunction, value.kind())
        }
    }

    pub fn expect_func<'a>(
        globals: &mut Globals,
        value: &'a Value,
    ) -> EvalResult<&'a Rc<Function>> {
        if let Value::Function(func) = value {
            Ok(func)
        } else {
            globals.set_kind_error(ValueKind::Function, value.kind())
        }
    }

    pub fn expect_exception_kind<'a>(
        globals: &mut Globals,
        value: &'a Value,
    ) -> EvalResult<&'a Rc<ExceptionKind>> {
        if let Value::ExceptionKind(exck) = value {
            Ok(exck)
        } else {
            globals.set_kind_error(ValueKind::ExceptionKind, value.kind())
        }
    }

    pub fn expect_cell<'a>(
        globals: &mut Globals,
        value: &'a Value,
    ) -> EvalResult<&'a Rc<RefCell<Value>>> {
        if let Value::Cell(cell) = value {
            Ok(cell)
        } else {
            globals.set_kind_error(ValueKind::Cell, value.kind())
        }
    }

    pub fn try_<T, E: Into<EvalError>>(
        globals: &mut Globals,
        result: Result<T, E>,
    ) -> EvalResult<T> {
        match result {
            Ok(t) => Ok(t),
            Err(error) => globals.set_exc_legacy(error.into()),
        }
    }

    pub fn osstr_to_str<'a>(
        globals: &mut Globals,
        osstr: &'a std::ffi::OsStr,
    ) -> EvalResult<&'a str> {
        match osstr.to_str() {
            Some(s) => Ok(s),
            None => globals.set_exc_legacy(EvalError::NotUnicode(osstr.to_owned())),
        }
    }

    pub fn truthy(globals: &mut Globals, value: &Value) -> EvalResult<bool> {
        Self::truthy0(globals, value, None)
    }

    /// Verison of truthy that will accept an optional debuginfo argument
    /// This is so that we only add debug information to the stacktrace if we need to make
    /// a jump. If we don't need to make a jump, the `globals.trace_push/trace_pop` might
    /// cost as much as the operation itself
    #[inline(always)]
    pub fn truthy0(
        globals: &mut Globals,
        value: &Value,
        _: Option<(&Code, usize)>,
    ) -> EvalResult<bool> {
        Ok(match value {
            Value::Uninitialized => return globals.set_exc_legacy(EvalError::UninitializedValue),
            Value::Nil => false,
            Value::Bool(x) => *x,
            Value::Int(x) => *x != 0,
            Value::Float(x) => *x != 0.0,
            Value::Symbol(x) => x.str().len() != 0,
            Value::String(x) => !x.is_empty(),
            Value::Bytes(x) => !x.is_empty(),
            Value::Path(_) => true,
            Value::List(x) => !x.is_empty(),
            Value::Table(x) => !x.is_empty(),
            Value::Set(x) => !x.is_empty(),
            Value::Map(x) => !x.is_empty(),
            Value::UserObject(_) => true,
            Value::Exception(_) => true,
            Value::NativeFunction(_) => true,
            Value::NativeClosure(_) => true,
            Value::Code(_) => true,
            Value::Function(_) => true,
            Value::Class(_) => true,
            Value::ExceptionKind(_) => true,
            Value::NativeIterator(_) => true,
            Value::GeneratorObject(_) => true,
            Value::Module(_) => true,
            Value::Handle(_) => true,
            Value::MutableString(x) => !x.borrow().is_empty(),
            Value::MutableBytes(x) => !x.borrow().is_empty(),
            Value::MutableList(x) => !x.borrow().is_empty(),
            Value::MutableSet(x) => !x.borrow().is_empty(),
            Value::MutableMap(x) => !x.borrow().is_empty(),
            Value::MutableUserObject(_) => true,
            Value::Cell(_) => true,
        })
    }

    pub fn eq(globals: &mut Globals, a: &Value, b: &Value) -> EvalResult<bool> {
        Self::eq0(globals, a, b, None)
    }
    /// Verison of eq that will accept an optional debuginfo argument
    /// This is so that we only add debug information to the stacktrace if we need to make
    /// a jump. If we don't need to make a jump, the `globals.trace_push/trace_pop` might
    /// cost as much as the comparison itself
    #[inline(always)]
    pub fn eq0(
        globals: &mut Globals,
        a: &Value,
        b: &Value,
        debuginfo: Option<(&Code, usize)>,
    ) -> EvalResult<bool> {
        Ok(match (a, b) {
            (Value::Uninitialized, Value::Uninitialized) => true,
            (Value::Nil, Value::Nil) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Int(a), Value::Float(b)) => (*a as f64) == *b,
            (Value::Float(a), Value::Int(b)) => *a == (*b as f64),
            (Value::Symbol(a), Value::Symbol(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Bytes(a), Value::Bytes(b)) => a == b,
            (Value::Path(a), Value::Path(b)) => a == b,
            (Value::List(a), Value::List(b)) => eq_list(globals, a, b, debuginfo)?,
            (Value::Table(a), Value::Table(b)) => {
                a.map().len() == b.map().len() && {
                    for (k, av) in a.map().iter() {
                        match b.map().get(k) {
                            Some(bv) => {
                                if !Self::eq0(globals, av, bv, debuginfo)? {
                                    return Ok(false);
                                }
                            }
                            None => return Ok(false),
                        }
                    }
                    true
                }
            }
            (Value::Set(a), Value::Set(b)) => eq_set(globals, a, b, debuginfo)?,
            (Value::Map(a), Value::Map(b)) => eq_map(globals, a, b, debuginfo)?,
            (Value::UserObject(_), _) => {
                a.is(b)
                    || Self::wrap_debuginfo(globals, debuginfo, |globals| {
                        let name = globals.symbol_dunder_eq();
                        let truthy = Eval::call_method(globals, name, vec![a.clone(), b.clone()])?;
                        Eval::truthy(globals, &truthy)
                    })?
            }
            (Value::MutableString(a), Value::MutableString(b)) => a == b,
            (Value::MutableBytes(a), Value::MutableBytes(b)) => a == b,
            (Value::MutableList(a), Value::MutableList(b)) => {
                eq_list(globals, &a.borrow(), &b.borrow(), debuginfo)?
            }
            (Value::MutableSet(a), Value::MutableSet(b)) => {
                eq_set(globals, &a.borrow(), &b.borrow(), debuginfo)?
            }
            (Value::MutableMap(a), Value::MutableMap(b)) => {
                eq_map(globals, &a.borrow(), &b.borrow(), debuginfo)?
            }
            (Value::MutableUserObject(_), _) => {
                a.is(b)
                    || Self::wrap_debuginfo(globals, debuginfo, |globals| {
                        let name = globals.symbol_dunder_eq();
                        let truthy = Eval::call_method(globals, name, vec![a.clone(), b.clone()])?;
                        Eval::truthy(globals, &truthy)
                    })?
            }
            _ => a.is(b),
        })
    }

    fn handle_unsupported_op<T>(
        globals: &mut Globals,
        debuginfo: Option<(&Code, usize)>,
        op: &str,
        args: Vec<&Value>,
    ) -> EvalResult<T> {
        if let Some((code, lineno)) = debuginfo {
            globals.trace_push(code.module_name().clone(), lineno);
        }
        globals.set_operand_type_error(op, args)
    }

    pub fn neg(globals: &mut Globals, x: &Value) -> EvalResult<Value> {
        Self::neg0(globals, x, None)
    }
    pub(crate) fn neg0(
        globals: &mut Globals,
        x: &Value,
        debuginfo: Option<(&Code, usize)>,
    ) -> EvalResult<Value> {
        Ok(match x {
            Value::Int(x) => Value::Int(-*x),
            Value::Float(x) => Value::Float(-*x),
            _ => return Self::handle_unsupported_op(globals, debuginfo, "-", vec![x]),
        })
    }

    pub fn pos(globals: &mut Globals, x: &Value) -> EvalResult<Value> {
        Self::pos0(globals, x, None)
    }
    pub(crate) fn pos0(
        globals: &mut Globals,
        x: &Value,
        debuginfo: Option<(&Code, usize)>,
    ) -> EvalResult<Value> {
        Ok(match x {
            Value::Int(x) => Value::Int(*x),
            _ => return Self::handle_unsupported_op(globals, debuginfo, "+", vec![x]),
        })
    }

    pub fn lt(globals: &mut Globals, a: &Value, b: &Value) -> EvalResult<bool> {
        Self::lt0(globals, a, b, None)
    }
    /// Verison of lt that will accept an optional debuginfo argument
    /// This is so that we only add debug information to the stacktrace if we need to make
    /// a jump. If we don't need to make a jump, the `globals.trace_push/trace_pop` might
    /// cost as much as the comparison itself
    #[inline(always)]
    pub(crate) fn lt0(
        globals: &mut Globals,
        a: &Value,
        b: &Value,
        debuginfo: Option<(&Code, usize)>,
    ) -> EvalResult<bool> {
        Ok(match (a, b) {
            (Value::Int(a), Value::Int(b)) => a < b,
            (Value::Float(a), Value::Float(b)) => a < b,
            (Value::Int(a), Value::Float(b)) => (*a as f64) < *b,
            (Value::Float(a), Value::Int(b)) => *a < (*b as f64),
            (Value::Symbol(a), Value::Symbol(b)) => a < b,
            (Value::String(a), Value::String(b)) => a < b,
            (Value::Bytes(a), Value::Bytes(b)) => a < b,
            (Value::Path(a), Value::Path(b)) => a < b,
            (Value::UserObject(_), _) => Self::wrap_debuginfo(globals, debuginfo, |globals| {
                let name = globals.symbol_dunder_lt();
                let truthy = Eval::call_method(globals, name, vec![a.clone(), b.clone()])?;
                Eval::truthy(globals, &truthy)
            })?,
            (Value::MutableUserObject(_), _) => {
                Self::wrap_debuginfo(globals, debuginfo, |globals| {
                    let name = globals.symbol_dunder_lt();
                    let truthy = Eval::call_method(globals, name, vec![a.clone(), b.clone()])?;
                    Eval::truthy(globals, &truthy)
                })?
            }
            _ => return Self::handle_unsupported_op(globals, debuginfo, "<", vec![a, b]),
        })
    }

    pub fn hash(globals: &mut Globals, x: &Value) -> EvalResult<u64> {
        Ok(match x {
            Value::Nil => compute_hash(()),
            Value::Bool(x) => compute_hash(x),
            Value::Int(x) => compute_int_hash(*x),
            Value::Float(x) => compute_float_hash(x),
            Value::Symbol(x) => compute_hash(x),
            Value::String(x) => compute_hash(x),
            Value::Path(x) => compute_hash(x),
            Value::List(x) => {
                let mut hasher = DefaultHasher::new();
                for item in x.iter() {
                    Eval::hash(globals, item)?.hash(&mut hasher);
                }
                hasher.finish()
            }
            Value::Table(x) => {
                let mut hasher = UnorderedHasher::new(x.len() as u64);
                for (key, val) in x.iter() {
                    hasher.add(compute_hash(key));
                    hasher.add(Eval::hash(globals, val)?);
                }
                hasher.finish()
            }
            Value::Set(x) => {
                let mut hasher = UnorderedHasher::new(x.len() as u64);
                for (key, ()) in x.iter() {
                    hasher.add(Eval::hash(globals, key)?);
                }
                hasher.finish()
            }
            Value::Map(x) => {
                let mut hasher = UnorderedHasher::new(x.len() as u64);
                for (key, val) in x.iter() {
                    hasher.add(Eval::hash(globals, key)?);
                    hasher.add(Eval::hash(globals, val)?);
                }
                hasher.finish()
            }
            Value::NativeFunction(x) => compute_hash(x.name()),
            Value::NativeClosure(x) => compute_hash(x.name()),
            Value::Module(x) => compute_hash(x.name()),
            Value::Code(x) => compute_hash(x.full_name()),
            Value::Function(f) => compute_hash(f.full_name()),
            Value::Class(cls) => compute_hash(cls.full_name()),
            Value::ExceptionKind(ek) => compute_hash(ek.id()),
            _ => {
                let exc = Exception::new(
                    globals.builtin_exceptions().HashError.clone(),
                    vec![format!("{:?}", x.kind()).into()],
                );
                globals.set_exc(exc)?
            }
        })
    }

    pub fn pow(globals: &mut Globals, a: Value, b: Value) -> EvalResult<Value> {
        Self::pow0(globals, a, b, None)
    }
    #[inline(always)]
    pub fn pow0(
        globals: &mut Globals,
        a: Value,
        b: Value,
        debuginfo: Option<(&Code, usize)>,
    ) -> EvalResult<Value> {
        Ok(match (a, b) {
            (Value::Int(a), Value::Int(b)) => {
                if b >= 0 && b <= (std::u32::MAX as i64) {
                    Value::Int(a.pow(b as u32))
                } else {
                    Value::Float((a as f64).powf(b as f64))
                }
            }
            (Value::Float(a), Value::Float(b)) => Value::Float(a.powf(b)),
            (Value::Int(a), Value::Float(b)) => Value::Float((a as f64).powf(b)),
            (Value::Float(a), Value::Int(b)) => Value::Float(a.powf(b as f64)),
            (a, b) => return Self::handle_unsupported_op(globals, debuginfo, "**", vec![&a, &b]),
        })
    }

    // 'add' accepts values rather than references because adding various
    // kinds of values (e.g. String, List, Table) can potentially be optimized
    // if the given value holds the sole reference
    pub fn add(globals: &mut Globals, a: Value, b: Value) -> EvalResult<Value> {
        Self::add0(globals, a, b, None)
    }
    #[inline(always)]
    pub fn add0(
        globals: &mut Globals,
        a: Value,
        b: Value,
        debuginfo: Option<(&Code, usize)>,
    ) -> EvalResult<Value> {
        Ok(match (a, b) {
            (Value::Int(a), Value::Int(b)) => Value::Int(a + b),
            (Value::Float(a), Value::Float(b)) => Value::Float(a + b),
            (Value::Int(a), Value::Float(b)) => Value::Float((a as f64) + b),
            (Value::Float(a), Value::Int(b)) => Value::Float(a + (b as f64)),
            (Value::String(a), Value::String(b)) => {
                let mut ab = RcStr::unwrap_or_clone(a);
                ab.push_str(b.str());
                Value::String(ab.into())
            }
            (Value::Bytes(a), Value::Bytes(b)) => {
                let mut ab = unwrap_or_clone_rc(a);
                ab.extend(b.iter());
                Value::Bytes(ab.into())
            }
            (Value::List(a), Value::List(b)) => {
                let mut ab = unwrap_or_clone_rc(a);
                ab.extend(b.iter().map(|v| v.clone()));
                ab.into()
            }
            (Value::Table(a), Value::Table(b)) => {
                let mut map = unwrap_or_clone_rc(a).map_move();
                map.extend(b.iter().map(|(k, v)| (*k, v.clone())));
                Value::Table(Table::new(map).into())
            }
            (Value::Set(a), Value::Set(b)) => {
                let mut set = unwrap_or_clone_rc(a);
                for (k, ()) in b.iter() {
                    set.s_insert(globals, k.clone(), ())?;
                }
                Value::Set(set.into())
            }
            (Value::Map(a), Value::Map(b)) => {
                let mut map = unwrap_or_clone_rc(a);
                for (k, v) in b.iter() {
                    map.s_insert(globals, k.clone(), v.clone())?;
                }
                Value::Map(map.into())
            }
            (a @ Value::UserObject(_), b) => Self::wrap_debuginfo(globals, debuginfo, |globals| {
                let name = globals.symbol_dunder_add();
                Eval::call_method(globals, name, vec![a, b])
            })?,
            (a @ Value::MutableUserObject(_), b) => {
                Self::wrap_debuginfo(globals, debuginfo, |globals| {
                    let name = globals.symbol_dunder_add();
                    Eval::call_method(globals, name, vec![a, b])
                })?
            }
            (a, b) => return Self::handle_unsupported_op(globals, debuginfo, "+", vec![&a, &b]),
        })
    }

    pub fn sub(globals: &mut Globals, a: Value, b: Value) -> EvalResult<Value> {
        Self::sub0(globals, a, b, None)
    }
    #[inline(always)]
    pub fn sub0(
        globals: &mut Globals,
        a: Value,
        b: Value,
        debuginfo: Option<(&Code, usize)>,
    ) -> EvalResult<Value> {
        Ok(match (a, b) {
            (Value::Int(a), Value::Int(b)) => Value::Int(a - b),
            (Value::Float(a), Value::Float(b)) => Value::Float(a - b),
            (Value::Int(a), Value::Float(b)) => Value::Float((a as f64) - b),
            (Value::Float(a), Value::Int(b)) => Value::Float(a - (b as f64)),
            (Value::Table(a), b) => {
                let mut map = unwrap_or_clone_rc(a).map_move();
                let iter = Self::iter(globals, &b)?;
                while let Some(key) = Self::next(globals, &iter)? {
                    let key = Self::expect_symbol(globals, &key)?;
                    map.remove(&key);
                }
                Value::Table(Table::new(map).into())
            }
            (a @ Value::UserObject(_), b) => Self::wrap_debuginfo(globals, debuginfo, |globals| {
                let name = globals.symbol_dunder_sub();
                Eval::call_method(globals, name, vec![a, b])
            })?,
            (a @ Value::MutableUserObject(_), b) => {
                Self::wrap_debuginfo(globals, debuginfo, |globals| {
                    let name = globals.symbol_dunder_sub();
                    Eval::call_method(globals, name, vec![a, b])
                })?
            }
            (a, b) => return Self::handle_unsupported_op(globals, debuginfo, "-", vec![&a, &b]),
        })
    }

    pub fn mul(globals: &mut Globals, a: Value, b: Value) -> EvalResult<Value> {
        Self::mul0(globals, a, b, None)
    }
    #[inline(always)]
    pub fn mul0(
        globals: &mut Globals,
        a: Value,
        b: Value,
        debuginfo: Option<(&Code, usize)>,
    ) -> EvalResult<Value> {
        Ok(match (a, b) {
            (Value::Int(a), Value::Int(b)) => Value::Int(a * b),
            (Value::Float(a), Value::Float(b)) => Value::Float(a * b),
            (Value::Int(a), Value::Float(b)) => Value::Float((a as f64) * b),
            (Value::Float(a), Value::Int(b)) => Value::Float(a * (b as f64)),
            (Value::String(s), Value::Int(n)) => {
                // TODO: consider throwing instead if n is < 0
                let n = std::cmp::max(0, n) as usize;
                s.repeat(n).into()
            }
            (Value::List(list), Value::Int(n)) => {
                Self::wrap_debuginfo(globals, debuginfo, |globals| {
                    let n = Self::expect_usize(globals, &Value::Int(n))?;
                    let mut ret = Vec::new();
                    for _ in 0..n {
                        ret.extend(list.iter().map(|v| v.clone()));
                    }
                    Ok(ret.into())
                })?
            }
            (Value::MutableList(list), Value::Int(n)) => {
                Self::wrap_debuginfo(globals, debuginfo, |globals| {
                    let n = Self::expect_usize(globals, &Value::Int(n))?;
                    let mut ret = Vec::new();
                    for _ in 0..n {
                        ret.extend(list.borrow().iter().map(|v| v.clone()));
                    }
                    Ok(Value::MutableList(RefCell::new(ret).into()))
                })?
            }
            (a @ Value::UserObject(_), b) => Self::wrap_debuginfo(globals, debuginfo, |globals| {
                let name = globals.symbol_dunder_mul();
                Eval::call_method(globals, name, vec![a, b])
            })?,
            (a @ Value::MutableUserObject(_), b) => {
                Self::wrap_debuginfo(globals, debuginfo, |globals| {
                    let name = globals.symbol_dunder_mul();
                    Eval::call_method(globals, name, vec![a, b])
                })?
            }
            (a, b) => return Self::handle_unsupported_op(globals, debuginfo, "*", vec![&a, &b]),
        })
    }

    pub fn div(globals: &mut Globals, a: Value, b: Value) -> EvalResult<Value> {
        Self::div0(globals, a, b, None)
    }
    #[inline(always)]
    pub fn div0(
        globals: &mut Globals,
        a: Value,
        b: Value,
        debuginfo: Option<(&Code, usize)>,
    ) -> EvalResult<Value> {
        Ok(match (a, b) {
            (Value::Int(a), Value::Int(b)) => Value::Float((a as f64) / (b as f64)),
            (Value::Float(a), Value::Float(b)) => Value::Float((a as f64) / (b as f64)),
            (Value::Float(a), Value::Int(b)) => Value::Float((a as f64) / (b as f64)),
            (Value::Int(a), Value::Float(b)) => Value::Float((a as f64) / (b as f64)),
            (a @ Value::UserObject(_), b) => Self::wrap_debuginfo(globals, debuginfo, |globals| {
                let name = globals.symbol_dunder_div();
                Eval::call_method(globals, name, vec![a, b])
            })?,
            (a @ Value::MutableUserObject(_), b) => {
                Self::wrap_debuginfo(globals, debuginfo, |globals| {
                    let name = globals.symbol_dunder_div();
                    Eval::call_method(globals, name, vec![a, b])
                })?
            }
            (a, b) => return Self::handle_unsupported_op(globals, debuginfo, "/", vec![&a, &b]),
        })
    }

    pub fn floordiv(globals: &mut Globals, a: Value, b: Value) -> EvalResult<Value> {
        Self::floordiv0(globals, a, b, None)
    }
    #[inline(always)]
    pub fn floordiv0(
        globals: &mut Globals,
        a: Value,
        b: Value,
        debuginfo: Option<(&Code, usize)>,
    ) -> EvalResult<Value> {
        Ok(match (a, b) {
            (Value::Int(a), Value::Int(b)) => Value::Int(divmod(a, b).0),
            (a, b) => return Self::handle_unsupported_op(globals, debuginfo, "fdiv", vec![&a, &b]),
        })
    }

    pub fn mod_(globals: &mut Globals, a: Value, b: Value) -> EvalResult<Value> {
        Self::mod0(globals, a, b, None)
    }
    #[inline(always)]
    pub fn mod0(
        globals: &mut Globals,
        a: Value,
        b: Value,
        debuginfo: Option<(&Code, usize)>,
    ) -> EvalResult<Value> {
        Ok(match (a, b) {
            (Value::Int(a), Value::Int(b)) => Value::Int(divmod(a, b).1),
            (a, b) => return Self::handle_unsupported_op(globals, debuginfo, "mod", vec![&a, &b]),
        })
    }

    pub fn truncdiv(globals: &mut Globals, a: Value, b: Value) -> EvalResult<Value> {
        Self::truncdiv0(globals, a, b, None)
    }
    #[inline(always)]
    pub fn truncdiv0(
        globals: &mut Globals,
        a: Value,
        b: Value,
        debuginfo: Option<(&Code, usize)>,
    ) -> EvalResult<Value> {
        Ok(match (a, b) {
            (Value::Int(a), Value::Int(b)) => Value::Int(a / b),
            (Value::Float(a), Value::Float(b)) => Value::Int((a / b).trunc() as i64),
            (Value::Int(a), Value::Float(b)) => Value::Int(((a as f64) / b).trunc() as i64),
            (Value::Float(a), Value::Int(b)) => Value::Int((a / (b as f64)).trunc() as i64),
            (a @ Value::UserObject(_), b) => Self::wrap_debuginfo(globals, debuginfo, |globals| {
                let name = globals.symbol_dunder_truncdiv();
                Eval::call_method(globals, name, vec![a, b])
            })?,
            (a @ Value::MutableUserObject(_), b) => {
                Self::wrap_debuginfo(globals, debuginfo, |globals| {
                    let name = globals.symbol_dunder_truncdiv();
                    Eval::call_method(globals, name, vec![a, b])
                })?
            }
            (a, b) => return Self::handle_unsupported_op(globals, debuginfo, "//", vec![&a, &b]),
        })
    }

    pub fn rem(globals: &mut Globals, a: Value, b: Value) -> EvalResult<Value> {
        Self::rem0(globals, a, b, None)
    }
    #[inline(always)]
    pub fn rem0(
        globals: &mut Globals,
        a: Value,
        b: Value,
        debuginfo: Option<(&Code, usize)>,
    ) -> EvalResult<Value> {
        Ok(match (a, b) {
            (Value::Int(a), Value::Int(b)) => Value::Int(a % b),
            (Value::Float(a), Value::Float(b)) => Value::Float(a % b),
            (Value::String(s), Value::List(args)) => {
                Self::wrap_debuginfo(globals, debuginfo, |globals| {
                    Eval::fmtstr(globals, s.str(), &args)
                })?
                .into()
            }
            (a @ Value::UserObject(_), b) => Self::wrap_debuginfo(globals, debuginfo, |globals| {
                let name = globals.symbol_dunder_rem();
                Eval::call_method(globals, name, vec![a, b])
            })?,
            (a @ Value::MutableUserObject(_), b) => {
                Self::wrap_debuginfo(globals, debuginfo, |globals| {
                    let name = globals.symbol_dunder_rem();
                    Eval::call_method(globals, name, vec![a, b])
                })?
            }
            (a, b) => return Self::handle_unsupported_op(globals, debuginfo, "%", vec![&a, &b]),
        })
    }

    fn wrap_debuginfo<F, R>(
        globals: &mut Globals,
        debuginfo: Option<(&Code, usize)>,
        f: F,
    ) -> EvalResult<R>
    where
        F: FnOnce(&mut Globals) -> EvalResult<R>,
    {
        if let Some((code, lineno)) = debuginfo {
            globals.trace_push(code.module_name().clone(), lineno);
            let r = f(globals)?;
            globals.trace_pop();
            Ok(r)
        } else {
            f(globals)
        }
    }

    pub fn call_method(globals: &mut Globals, name: Symbol, args: Vec<Value>) -> EvalResult<Value> {
        let cls = Self::classof(globals, &args[0])?.clone();
        match cls.get_from_instance_map(&name) {
            Some(method) => Self::call(globals, method, args),
            None => globals.set_exc_legacy(EvalError::NoSuchMethod(name.str().into(), cls)),
        }
    }

    pub fn call(globals: &mut Globals, f: &Value, args: Vec<Value>) -> EvalResult<Value> {
        Self::call_with_kwargs(globals, f, args, None)
    }

    fn user_defined_instantiate0(
        globals: &mut Globals,
        f: &Value,
        args: Vec<Value>,
        kwargs: Option<HashMap<Symbol, Value>>,
    ) -> EvalResult<Value> {
        let f = Self::get_static_attr_or_err(globals, f, Symbol::DUNDER_CALL)?;
        Self::call_with_kwargs(globals, &f, args, kwargs)
    }

    fn user_defined_instantiate(
        globals: &mut Globals,
        cls: &Rc<Class>,
        args: Vec<Value>,
        kwargs: Option<HashMap<Symbol, Value>>,
    ) -> EvalResult<Value> {
        globals.push_new_stack(cls.clone());
        let f = Value::Class(cls.clone());
        let r = Self::user_defined_instantiate0(globals, &f, args, kwargs);
        globals.pop_new_stack();
        r
    }

    pub fn call_with_kwargs(
        globals: &mut Globals,
        f: &Value,
        args: Vec<Value>,
        kwargs: Option<HashMap<Symbol, Value>>,
    ) -> EvalResult<Value> {
        Ok(match f {
            Value::NativeFunction(f) => f.apply_with_kwargs(globals, args, kwargs)?,
            Value::NativeClosure(f) => f.apply_with_kwargs(globals, args, kwargs)?,
            Value::Function(f) => f.apply_with_kwargs(globals, args, kwargs)?,
            Value::Class(cls) => match cls.kind() {
                ClassKind::NativeClass
                | ClassKind::Trait
                | ClassKind::UserDefinedClass
                | ClassKind::UserDefinedMutableClass => {
                    Self::user_defined_instantiate(globals, cls, args, kwargs)?
                }
                ClassKind::UserDefinedCaseClass => {
                    if cls.has_static_call() {
                        Self::user_defined_instantiate(globals, cls, args, kwargs)?
                    } else {
                        Class::instantiate(cls, globals, args, kwargs)?.into()
                    }
                }
            },
            Value::ExceptionKind(exck) => {
                let (mut args, _) = match exck.fields_as_parameter_info().translate(args, kwargs) {
                    Ok(pair) => pair,
                    Err(error) => {
                        return globals.set_exc_legacy(error.into());
                    }
                };
                if let Some(Value::Uninitialized) = args.last() {
                    args.pop().unwrap();
                }
                Value::Exception(Exception::new(exck.clone(), args).into())
            }
            _ => return globals.set_exc_legacy(EvalError::NotCallable(f.clone())),
        })
    }

    pub fn mcall(
        globals: &mut Globals,
        owner: &Value,
        name: Symbol,
        mut args: Vec<Value>,
    ) -> EvalResult<Value> {
        let method = Self::get_method(globals, owner, name)?.clone();
        args.insert(0, owner.clone());
        Self::call(globals, &method, args)
    }

    pub fn get_method(globals: &mut Globals, owner: &Value, name: Symbol) -> EvalResult<Value> {
        let cls = Self::classof(globals, owner)?;
        match cls.get_from_instance_map(&name) {
            Some(method) => Ok(method.clone()),
            None => {
                let cls = cls.clone();
                let name = RcStr::from(name);
                globals.set_exc_legacy(EvalError::NoSuchMethod(name, cls))
            }
        }
    }

    pub fn sort(globals: &mut Globals, vec: &mut [Value]) -> EvalResult<()> {
        crate::gsort(globals, vec, Eval::lt)
    }

    pub fn str(globals: &mut Globals, value: &Value) -> EvalResult<RcStr> {
        Ok(match value {
            Value::Symbol(x) => RcStr::from(*x),
            Value::String(x) => x.clone(),
            Value::Path(x) => match x.to_str() {
                Some(s) => s.into(),
                None => {
                    let os_string = (**x).to_owned().into_os_string();
                    return globals.set_exc_legacy(EvalError::NotUnicode(os_string));
                }
            },
            Value::UserObject(x) => {
                let cls = x.cls();
                if let Some(mt) = cls.get_from_instance_map(&globals.symbol_dunder_str()) {
                    let strval = Self::call(globals, mt, vec![value.clone()])?;
                    Self::expect_string(globals, &strval)?.clone()
                } else if let Some(mt) = cls.get_from_instance_map(&globals.symbol_dunder_repr()) {
                    let strval = Self::call(globals, mt, vec![value.clone()])?;
                    Self::expect_string(globals, &strval)?.clone()
                } else {
                    format!("{:?}", x).into()
                }
            }
            Value::MutableUserObject(x) => {
                let cls = x.cls();
                if let Some(mt) = cls.get_from_instance_map(&globals.symbol_dunder_str()) {
                    let strval = Self::call(globals, mt, vec![value.clone()])?;
                    Self::expect_string(globals, &strval)?.clone()
                } else if let Some(mt) = cls.get_from_instance_map(&globals.symbol_dunder_repr()) {
                    let strval = Self::call(globals, mt, vec![value.clone()])?;
                    Self::expect_string(globals, &strval)?.clone()
                } else {
                    format!("{:?}", x).into()
                }
            }
            Value::Exception(x) => format!("{}", x).into(),
            _ => Self::repr(globals, value)?,
        })
    }

    pub fn repr(globals: &mut Globals, value: &Value) -> EvalResult<RcStr> {
        Ok(match value {
            Value::Uninitialized => return globals.set_exc_legacy(EvalError::UninitializedValue),
            Value::Nil => "nil".into(),
            Value::Bool(x) => if *x { "true" } else { "false" }.into(),
            Value::Int(x) => format!("{}", x).into(),
            Value::Float(x) => format!("{}", x).into(),
            Value::Symbol(x) => format!(":{}", x.str()).into(),
            Value::String(x) => reprstr(x).into(),
            Value::Bytes(x) => format!("Bytes({:?})", x).into(),
            Value::Path(x) => format!("Path::new({:?})", x).into(),
            Value::List(x) => list2str(globals, &*x)?.into(),
            Value::Table(x) => table2str(globals, x.map())?.into(),
            Value::Set(x) => set2str(globals, "Set", &*x)?.into(),
            Value::Map(x) => map2str(globals, &*x)?.into(),
            Value::UserObject(x) => {
                let cls = x.cls();
                if let Some(mt) = cls.get_from_instance_map(&globals.symbol_dunder_repr()) {
                    let reprval = Self::call(globals, mt, vec![value.clone()])?;
                    Self::expect_string(globals, &reprval)?.clone()
                } else {
                    format!("{:?}", x).into()
                }
            }
            Value::Exception(x) => format!("{:?}", x).into(),
            Value::NativeFunction(f) => format!("{:?}", f).into(),
            Value::NativeClosure(f) => format!("{:?}", f).into(),
            Value::Code(x) => format!("{:?}", x).into(),
            Value::Function(f) => format!("{:?}", f).into(),
            Value::Class(c) => format!("{:?}", c).into(),
            Value::ExceptionKind(c) => format!("{:?}", c).into(),
            Value::NativeIterator(iter) => format!("{:?}", iter.borrow()).into(),
            Value::GeneratorObject(obj) => format!("{:?}", obj.borrow()).into(),
            Value::Module(m) => format!("{:?}", m).into(),
            Value::Handle(handle) => format!("{:?}", handle).into(),
            Value::MutableString(x) => format!("@{}", reprstr(&x.borrow())).into(),
            Value::MutableBytes(x) => format!("MutableBytes({:?})", &x.borrow()).into(),
            Value::MutableList(x) => format!("@{}", list2str(globals, &x.borrow())?).into(),
            Value::MutableSet(x) => set2str(globals, "MutableSet", &x.borrow())?.into(),
            Value::MutableMap(x) => format!("@{}", map2str(globals, &x.borrow())?).into(),
            Value::MutableUserObject(x) => {
                let cls = x.cls();
                if let Some(mt) = cls.get_from_instance_map(&globals.symbol_dunder_repr()) {
                    let reprval = Self::call(globals, mt, vec![value.clone()])?;
                    Self::expect_string(globals, &reprval)?.clone()
                } else {
                    format!("{:?}", x).into()
                }
            }
            Value::Cell(x) => format!("<cell {}>", Self::repr(globals, &x.borrow())?).into(),
        })
    }

    pub fn getattr(_: &mut Globals, owner: &Value, name: Symbol) -> Option<Value> {
        match owner {
            Value::Table(table) => table.get(name).cloned(),
            Value::UserObject(obj) => obj.get(name).cloned(),
            Value::MutableUserObject(obj) => obj.get(name).map(|rfc| rfc.borrow().clone()),
            _ => return None,
        }
    }

    pub fn setattr(_: &mut Globals, owner: &Value, name: Symbol, value: Value) -> Result<(), ()> {
        match owner {
            Value::MutableUserObject(obj) => match obj.get(name) {
                Some(rfc) => {
                    *rfc.borrow_mut() = value;
                    Ok(())
                }
                None => {
                    println!("OBJ -> {:?}", obj);
                    Err(())
                }
            },
            _ => Err(()),
        }
    }

    pub fn get_static_attr(_: &mut Globals, owner: &Value, name: Symbol) -> Option<Value> {
        match owner {
            Value::Class(cls) => cls.get_static(&name).cloned(),
            Value::Module(m) => m.get(&name),
            _ => return None,
        }
    }

    pub fn get_static_attr_or_err(
        globals: &mut Globals,
        owner: &Value,
        name: Symbol,
    ) -> EvalResult<Value> {
        match Self::get_static_attr(globals, owner, name) {
            Some(attr) => Ok(attr),
            None => globals.set_static_attr_error(name, owner.clone()),
        }
    }

    pub fn call_static_attr(
        globals: &mut Globals,
        owner: &Value,
        name: Symbol,
        args: Vec<Value>,
    ) -> EvalResult<Value> {
        let f = Self::get_static_attr_or_err(globals, owner, name)?;
        Self::call(globals, &f, args)
    }

    pub fn iter(globals: &mut Globals, iterable: &Value) -> EvalResult<Value> {
        match iterable {
            Value::Bytes(bytes) => Ok(iterbytes(globals, bytes.clone()).into()),
            Value::List(list) => Ok(iterlist(globals, list.clone()).into()),
            Value::Set(set) => Ok(iterset(globals, set.clone()).into()),
            Value::Map(map) => Ok(itermap(globals, map.clone()).into()),
            Value::NativeIterator(_) => Ok(iterable.clone()),
            Value::GeneratorObject(_) => Ok(iterable.clone()),
            _ => {
                let kind = iterable.kind();
                return globals.set_exc_legacy(EvalError::ExpectedIterable(kind));
            }
        }
    }

    pub fn next(globals: &mut Globals, iterator: &Value) -> EvalResult<Option<Value>> {
        match iterator {
            // NOTE: the iterators are borrowed mutably, so if you try to
            // call 'next' on it while 'next' is still running on it, we will
            // Rust panic. It is good that it crashes (recursively calling next on
            // a generator is almost certainly a bug), but we might want to crash more
            // gently with a nicer error message at some point.
            Value::NativeIterator(iter) => iter.borrow_mut().next(globals),
            Value::GeneratorObject(obj) => obj.borrow_mut().next(globals),
            _ => {
                let kind = iterator.kind();
                return globals.set_exc_legacy(EvalError::ExpectedIterator(kind));
            }
        }
    }

    pub fn resume(globals: &mut Globals, iterator: &Value, value: Value) -> GeneratorResult {
        match iterator {
            // NOTE: the iterators are borrowed mutably, so if you try to
            // call 'resume' on it while 'resume' is still running on it, we will
            // Rust panic. It is good that it crashes (recursively calling resume on
            // a generator is almost certainly a bug), but we might want to crash more
            // gently with a nicer error message at some point.
            Value::NativeIterator(iter) => iter.borrow_mut().resume(globals, value),
            Value::GeneratorObject(obj) => obj.borrow_mut().resume(globals, value),
            _ => {
                let kind = iterator.kind();
                assert!(globals
                    .set_exc_legacy::<()>(EvalError::ExpectedIterator(kind))
                    .is_err());
                return GeneratorResult::Error;
            }
        }
    }

    pub fn bytes_from_iterable(globals: &mut Globals, iterable: &Value) -> EvalResult<Value> {
        if let Value::Bytes(_) = &iterable {
            Ok(iterable.clone())
        } else {
            let iterator = Self::iter(globals, iterable)?;
            let mut bytes = Vec::new();
            while let Some(item) = Self::next(globals, &iterator)? {
                Self::add_bytes(globals, &mut bytes, &item)?;
            }
            Ok(bytes.into())
        }
    }

    pub fn list_from_iterable(globals: &mut Globals, iterable: &Value) -> EvalResult<Value> {
        if let Value::List(_) = &iterable {
            Ok(iterable.clone())
        } else {
            Ok(Self::iterable_to_vec(globals, iterable)?.into())
        }
    }

    pub fn table_from_iterable(globals: &mut Globals, pairs: &Value) -> EvalResult<Value> {
        let iterator = Self::iter(globals, pairs)?;
        let mut map = HashMap::new();
        while let Some(pair) = Self::next(globals, &iterator)? {
            let (key, val) = Self::unpack_pair(globals, &pair)?;
            let key = Self::expect_symbol(globals, &key)?;
            map.insert(key, val);
        }
        Ok(Value::Table(Table::new(map).into()))
    }

    pub fn set_from_iterable(globals: &mut Globals, iterable: &Value) -> EvalResult<Value> {
        if let Value::Set(_) = iterable {
            Ok(iterable.clone())
        } else {
            Ok(Self::iterable_to_vset(globals, iterable)?.into())
        }
    }

    pub fn map_from_iterable(globals: &mut Globals, pairs: &Value) -> EvalResult<Value> {
        if let Value::Map(_) = pairs {
            Ok(pairs.clone())
        } else {
            Ok(Self::iterable_to_vmap(globals, pairs)?.into())
        }
    }

    pub fn mutable_list_from_iterable(
        globals: &mut Globals,
        iterable: &Value,
    ) -> EvalResult<Value> {
        Ok(Value::MutableList(
            RefCell::new(Self::iterable_to_vec(globals, iterable)?.into()).into(),
        ))
    }

    pub fn mutable_set_from_iterable(globals: &mut Globals, iterable: &Value) -> EvalResult<Value> {
        let vset = Self::iterable_to_vset(globals, iterable)?;
        Ok(Value::MutableSet(RefCell::new(vset).into()))
    }

    pub fn mutable_map_from_iterable(globals: &mut Globals, iterable: &Value) -> EvalResult<Value> {
        let vmap = Self::iterable_to_vmap(globals, iterable)?;
        Ok(Value::MutableMap(RefCell::new(vmap).into()))
    }

    pub fn from_iterable(
        globals: &mut Globals,
        type_: &Value,
        iterable: Value,
    ) -> EvalResult<Value> {
        Self::call_static_attr(globals, type_, Symbol::FROM_ITERABLE, vec![iterable])
    }

    pub fn extend_from_iterable(
        globals: &mut Globals,
        vec: &mut Vec<Value>,
        iterable: &Value,
    ) -> EvalResult<()> {
        if let Value::List(list) = iterable {
            vec.extend(list.iter().map(|v| v.clone()));
        } else {
            let iterator = Self::iter(globals, iterable)?;
            while let Some(next) = Self::next(globals, &iterator)? {
                vec.push(next);
            }
        }
        Ok(())
    }

    pub fn extend_str(globals: &mut Globals, s: &mut String, strlike: &Value) -> EvalResult<()> {
        match strlike {
            Value::Symbol(sym) => s.push_str(sym.str()),
            Value::String(other) => s.push_str(other),
            Value::Path(path) => match path.to_str() {
                Some(other) => s.push_str(other),
                None => {
                    let os_string = (**path).to_owned().into_os_string();
                    return globals.set_exc_legacy(EvalError::NotUnicode(os_string));
                }
            },
            Value::MutableString(other) => {
                s.push_str(&other.borrow());
            }
            _ => {
                Self::expect_string(globals, strlike)?;
                assert!(false);
            }
        }
        Ok(())
    }

    pub fn iterable_to_vec(globals: &mut Globals, iterable: &Value) -> EvalResult<Vec<Value>> {
        let iterator = Self::iter(globals, iterable)?;
        let mut ret = Vec::new();
        while let Some(next) = Self::next(globals, &iterator)? {
            ret.push(next);
        }
        Ok(ret)
    }

    pub fn iterable_to_vset(globals: &mut Globals, iterable: &Value) -> EvalResult<VSet> {
        let iterator = Self::iter(globals, iterable)?;
        let mut set = VSet::new();
        while let Some(key) = Self::next(globals, &iterator)? {
            set.s_insert(globals, key, ())?;
        }
        Ok(set)
    }

    pub fn iterable_to_vmap(globals: &mut Globals, pairs: &Value) -> EvalResult<VMap> {
        let iterator = Self::iter(globals, pairs)?;
        let mut map = VMap::new();
        while let Some(pair) = Self::next(globals, &iterator)? {
            let (key, val) = Self::unpack_pair(globals, &pair)?;
            map.s_insert(globals, key, val)?;
        }
        Ok(map)
    }

    pub fn unpack(globals: &mut Globals, iterable: &Value, n: usize) -> EvalResult<Vec<Value>> {
        let iterator = Self::iter(globals, iterable)?;
        let mut ret = Vec::new();
        while let Some(next) = Self::next(globals, &iterator)? {
            ret.push(next);
        }
        if ret.len() != n {
            return globals.set_exc(Exception::new(
                globals.builtin_exceptions().UnpackError.clone(),
                vec![Value::Int(n as i64), Value::Int(ret.len() as i64)],
            ));
        }
        Ok(ret)
    }

    pub fn unpack_pair(globals: &mut Globals, iterable: &Value) -> EvalResult<(Value, Value)> {
        let mut pair = Self::unpack(globals, iterable, 2)?.into_iter();
        let first = pair.next().unwrap();
        let second = pair.next().unwrap();
        Ok((first, second))
    }

    pub fn unpack_triple(
        globals: &mut Globals,
        iterable: &Value,
    ) -> EvalResult<(Value, Value, Value)> {
        let mut triple = Self::unpack(globals, iterable, 3)?.into_iter();
        let first = triple.next().unwrap();
        let second = triple.next().unwrap();
        let third = triple.next().unwrap();
        Ok((first, second, third))
    }

    pub fn unpack4(
        globals: &mut Globals,
        iterable: &Value,
    ) -> EvalResult<(Value, Value, Value, Value)> {
        let mut quadruple = Self::unpack(globals, iterable, 4)?.into_iter();
        let first = quadruple.next().unwrap();
        let second = quadruple.next().unwrap();
        let third = quadruple.next().unwrap();
        let fourth = quadruple.next().unwrap();
        Ok((first, second, third, fourth))
    }

    pub fn fmtstr(globals: &mut Globals, fmt: &str, args: &Vec<Value>) -> EvalResult<String> {
        enum State {
            Normal,
            Escape,
        }
        fn next<'a>(
            globals: &mut Globals,
            args: &'a Vec<Value>,
            i: &mut usize,
        ) -> EvalResult<&'a Value> {
            match args.get(*i) {
                Some(arg) => {
                    *i += 1;
                    Ok(arg)
                }
                None => globals.set_exc_str(&format!(
                    "Expected at least {} args, but got only {}",
                    *i + 1,
                    args.len(),
                )),
            }
        }
        let mut state = State::Normal;
        let mut s = String::new();
        let mut i = 0;
        for c in fmt.chars() {
            match state {
                State::Normal => match c {
                    '%' => state = State::Escape,
                    c => s.push(c),
                },
                State::Escape => match c {
                    '%' => {
                        s.push('%');
                        state = State::Normal;
                    }
                    's' => {
                        let arg = next(globals, args, &mut i)?;
                        let argstr = Eval::str(globals, arg)?;
                        s.push_str(argstr.str());
                        state = State::Normal;
                    }
                    'r' => {
                        let arg = next(globals, args, &mut i)?;
                        let argstr = Eval::repr(globals, arg)?;
                        s.push_str(argstr.str());
                        state = State::Normal;
                    }
                    c => {
                        return globals.set_exc_str(&format!("Invalid escape char {:?}", c,));
                    }
                },
            }
        }
        if let State::Escape = state {
            return globals.set_exc_str(&format!("Unterminated escape in format string"));
        }
        if i < args.len() {
            return globals.set_exc_str(&format!(
                "Too many arguments provided ({} given, {} used)",
                args.len(),
                i,
            ));
        }
        Ok(s)
    }

    pub fn parameter_info_to_value(_: &mut Globals, pi: &ParameterInfo) -> EvalResult<Value> {
        let req: Vec<Value> = pi.required().iter().map(|s| Value::from(*s)).collect();
        let opt: Vec<Value> = pi
            .optional()
            .iter()
            .map(|(s, v)| Value::from(vec![Value::from(*s), v.clone()]))
            .collect();
        let var: Value = pi.variadic().clone().map(Value::from).unwrap_or(Value::Nil);
        let kw: Value = pi.keywords().clone().map(Value::from).unwrap_or(Value::Nil);
        Ok(vec![Value::from(req), Value::from(opt), var, kw].into())
    }
}

fn unwrap_or_clone_rc<T: Clone>(rc: Rc<T>) -> T {
    match Rc::try_unwrap(rc) {
        Ok(t) => t,
        Err(rc) => (*rc).clone(),
    }
}

fn eq_list(
    globals: &mut Globals,
    a: &Vec<Value>,
    b: &Vec<Value>,
    debuginfo: Option<(&Code, usize)>,
) -> EvalResult<bool> {
    Ok(a.len() == b.len() && {
        for (x, y) in a.iter().zip(b.iter()) {
            if !Eval::eq0(globals, x, y, debuginfo)? {
                return Ok(false);
            }
        }
        true
    })
}

fn eq_set(
    globals: &mut Globals,
    a: &VSet,
    b: &VSet,
    _debuginfo: Option<(&Code, usize)>,
) -> EvalResult<bool> {
    Ok(a.len() == b.len() && {
        for (key, ()) in a.iter() {
            if let Some(()) = b.s_get(globals, key)? {
                // key in a found in b
            } else {
                return Ok(false);
            }
        }
        true
    })
}

fn eq_map(
    globals: &mut Globals,
    a: &VMap,
    b: &VMap,
    debuginfo: Option<(&Code, usize)>,
) -> EvalResult<bool> {
    Ok(a.len() == b.len() && {
        for (key, a_value) in a.iter() {
            if let Some(b_value) = b.s_get(globals, key)? {
                if !Eval::eq0(globals, a_value, b_value, debuginfo)? {
                    return Ok(false);
                }
            } else {
                return Ok(false);
            }
        }
        true
    })
}

fn reprstr(s: &str) -> String {
    let mut ret = String::new();
    ret.push('"');
    for c in s.chars() {
        match c {
            '"' => ret.push_str("\\\""),
            '\\' => ret.push_str("\\\\"),
            '\n' => ret.push_str("\\n"),
            '\r' => ret.push_str("\\r"),
            '\t' => ret.push_str("\\t"),
            _ if c.is_control() => ret.push_str(&format!("\\u{{{:X}}}", c as u32)),
            _ => ret.push(c),
        }
    }
    ret.push('"');
    ret
}

fn list2str(globals: &mut Globals, vec: &Vec<Value>) -> EvalResult<String> {
    let mut ret = String::new();
    ret.push('[');
    let mut first = true;
    for item in vec.iter() {
        if !first {
            ret.push_str(", ");
        }
        ret.push_str(&*Eval::repr(globals, item)?);
        first = false;
    }
    ret.push(']');
    Ok(ret)
}

fn set2str(globals: &mut Globals, prefix: &str, set: &VSet) -> EvalResult<String> {
    let mut ret = String::new();
    ret.push_str(prefix);
    ret.push_str("([");
    if set.is_empty() {
        ret.push(':');
    } else {
        let mut first = true;
        for (key, ()) in set.iter() {
            if !first {
                ret.push_str(", ");
            }
            ret.push_str(&*Eval::repr(globals, key)?);
            first = false;
        }
    }
    ret.push_str("])");
    Ok(ret)
}

fn map2str(globals: &mut Globals, map: &VMap) -> EvalResult<String> {
    let mut ret = String::new();
    ret.push('[');
    if map.is_empty() {
        ret.push(':');
    } else {
        let mut first = true;
        for (key, value) in map.iter() {
            if !first {
                ret.push_str(", ");
            }
            ret.push_str(&*Eval::repr(globals, key)?);
            ret.push_str(": ");
            ret.push_str(&*Eval::repr(globals, value)?);
            first = false;
        }
    }
    ret.push(']');
    Ok(ret)
}

fn table2str(globals: &mut Globals, table: &HashMap<Symbol, Value>) -> EvalResult<String> {
    // sort the entries to ensure that when you print it, it always looks the same
    let mut pairs: Vec<_> = table.iter().collect();
    pairs.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
    let mut ret = String::new();
    ret.push_str("Table(");
    let mut first = true;
    for (key, value) in pairs {
        if !first {
            ret.push_str(", ");
        }
        ret.push_str(key.str());
        ret.push_str("=");
        ret.push_str(&*Eval::repr(globals, value)?);
        first = false;
    }
    ret.push_str(")");
    Ok(ret)
}

fn iterbytes(_: &mut Globals, bytes: Rc<Vec<u8>>) -> NativeIterator {
    let mut i = 0;
    NativeIterator::new(move |_, _| {
        if i < bytes.len() {
            i += 1;
            GeneratorResult::Yield(((*bytes)[i - 1] as i64).into())
        } else {
            GeneratorResult::Done(Value::Nil)
        }
    })
}

fn iterlist(_: &mut Globals, list: Rc<Vec<Value>>) -> NativeIterator {
    // TODO: Figure out how to hold iterators with lifetimes tied to original
    // value inside a Value
    let mut i = 0;
    NativeIterator::new(move |_, _| {
        if i < list.len() {
            i += 1;
            GeneratorResult::Yield((*list)[i - 1].clone())
        } else {
            GeneratorResult::Done(Value::Nil)
        }
    })
}

fn iterset(_: &mut Globals, set: Rc<VSet>) -> NativeIterator {
    let mut i = 0;
    let len = set.reserved_entries_count();
    NativeIterator::new(move |_, _| {
        while i < len {
            i += 1;
            if let Some((key, ())) = set.get_pair_at_index(i - 1) {
                return GeneratorResult::Yield(key.clone());
            }
        }
        GeneratorResult::Done(Value::Nil)
    })
}

fn itermap(_: &mut Globals, set: Rc<VMap>) -> NativeIterator {
    let mut i = 0;
    let len = set.reserved_entries_count();
    NativeIterator::new(move |_, _| {
        while i < len {
            i += 1;
            if let Some((key, val)) = set.get_pair_at_index(i - 1) {
                return GeneratorResult::Yield(vec![key.clone(), val.clone()].into());
            }
        }
        GeneratorResult::Done(Value::Nil)
    })
}

fn compute_hash<T: Hash>(t: T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

fn compute_int_hash(i: i64) -> u64 {
    compute_hash(i)
}

fn compute_float_hash(f: &f64) -> u64 {
    if f.fract() == 0.0 {
        compute_int_hash(*f as i64)
    } else {
        compute_hash(f.to_bits())
    }
}

impl FailableHash<Globals, Value, ErrorIndicator> for Eval {
    fn hash(globals: &mut Globals, value: &Value) -> EvalResult<u64> {
        Eval::hash(globals, value)
    }
}

impl FailableEq<Globals, Value, ErrorIndicator> for Eval {
    fn eq(globals: &mut Globals, a: &Value, b: &Value) -> EvalResult<bool> {
        Eval::eq(globals, a, b)
    }
}

pub type VMap = GMap<Globals, Value, Value, Eval, Eval, ErrorIndicator>;
pub type VSet = GMap<Globals, Value, (), Eval, Eval, ErrorIndicator>;

impl VMap {
    pub fn to_string_keys(&self, globals: &mut Globals) -> EvalResult<HashMap<RcStr, Value>> {
        let mut ret = HashMap::new();
        for (k, v) in self {
            let k = Eval::expect_string(globals, k)?;
            ret.insert(k.clone(), v.clone());
        }
        Ok(ret)
    }
}
