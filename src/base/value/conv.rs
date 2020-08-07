use super::*;
use std::ffi::OsStr;
use std::ffi::OsString;

impl From<()> for Value {
    fn from(_: ()) -> Self {
        Self::Nil
    }
}

impl From<bool> for Value {
    fn from(x: bool) -> Self {
        Self::Bool(x)
    }
}

impl From<i64> for Value {
    fn from(x: i64) -> Self {
        Self::Number(x as f64)
    }
}

impl From<i32> for Value {
    fn from(x: i32) -> Self {
        Self::Number(x as f64)
    }
}

impl From<i16> for Value {
    fn from(x: i16) -> Self {
        Self::Number(x as f64)
    }
}

impl From<i8> for Value {
    fn from(x: i8) -> Self {
        Self::Number(x as f64)
    }
}

impl From<u64> for Value {
    fn from(x: u64) -> Self {
        Self::Number(x as f64)
    }
}

impl From<u32> for Value {
    fn from(x: u32) -> Self {
        Self::Number(x as f64)
    }
}

impl From<u16> for Value {
    fn from(x: u16) -> Self {
        Self::Number(x as f64)
    }
}

impl From<u8> for Value {
    fn from(x: u8) -> Self {
        Self::Number(x as f64)
    }
}

impl From<usize> for Value {
    fn from(x: usize) -> Self {
        Self::Number(x as f64)
    }
}

impl From<isize> for Value {
    fn from(x: isize) -> Self {
        Self::Number(x as f64)
    }
}

impl From<f32> for Value {
    fn from(x: f32) -> Self {
        Self::Number(x as f64)
    }
}

impl From<f64> for Value {
    fn from(x: f64) -> Self {
        Self::Number(x)
    }
}

impl From<char> for Value {
    fn from(c: char) -> Self {
        Self::String(format!("{}", c).into())
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Self::String(s.into())
    }
}

impl From<&String> for Value {
    fn from(s: &String) -> Self {
        Self::String(s.into())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Self::String(s.into())
    }
}

impl TryFrom<OsString> for Value {
    type Error = Error;
    fn try_from(s: OsString) -> Result<Self> {
        Ok(Self::String(RcStr::try_from(s)?))
    }
}

impl TryFrom<&OsString> for Value {
    type Error = Error;
    fn try_from(s: &OsString) -> Result<Self> {
        Ok(Self::String(RcStr::try_from(s)?))
    }
}

impl TryFrom<&OsStr> for Value {
    type Error = Error;
    fn try_from(s: &OsStr) -> Result<Self> {
        Ok(Self::String(RcStr::try_from(s)?))
    }
}

impl From<RcStr> for Value {
    fn from(s: RcStr) -> Self {
        Self::String(s)
    }
}

impl From<&RcStr> for Value {
    fn from(s: &RcStr) -> Self {
        Self::String(s.clone())
    }
}

impl From<Rc<Function>> for Value {
    fn from(f: Rc<Function>) -> Self {
        Self::Function(f)
    }
}

impl From<&Rc<Function>> for Value {
    fn from(f: &Rc<Function>) -> Self {
        Self::Function(f.clone())
    }
}

impl From<Function> for Value {
    fn from(f: Function) -> Self {
        Self::Function(f.into())
    }
}

impl From<Rc<NativeFunction>> for Value {
    fn from(f: Rc<NativeFunction>) -> Self {
        Self::NativeFunction(f)
    }
}

impl From<&Rc<NativeFunction>> for Value {
    fn from(f: &Rc<NativeFunction>) -> Self {
        Self::NativeFunction(f.clone())
    }
}

impl From<NativeFunction> for Value {
    fn from(f: NativeFunction) -> Self {
        Self::NativeFunction(f.into())
    }
}

impl From<Generator> for Value {
    fn from(gen: Generator) -> Self {
        Self::Generator(Rc::new(RefCell::new(gen)))
    }
}

impl From<NativeGenerator> for Value {
    fn from(gen: NativeGenerator) -> Self {
        Self::NativeGenerator(Rc::new(RefCell::new(gen)))
    }
}

impl From<Rc<Class>> for Value {
    fn from(cls: Rc<Class>) -> Self {
        Self::Class(cls)
    }
}

impl From<&Rc<Class>> for Value {
    fn from(cls: &Rc<Class>) -> Self {
        Self::Class(cls.clone())
    }
}

impl From<Rc<Module>> for Value {
    fn from(m: Rc<Module>) -> Self {
        Self::Module(m)
    }
}

impl From<&Rc<Module>> for Value {
    fn from(m: &Rc<Module>) -> Self {
        Self::Module(m.clone())
    }
}

impl TryFrom<&Value> for Key {
    type Error = Error;
    fn try_from(value: &Value) -> Result<Key> {
        Key::try_from(value.clone())
    }
}

impl TryFrom<Value> for Key {
    type Error = Error;
    fn try_from(value: Value) -> Result<Key> {
        match value {
            Value::Nil => Ok(Key::Nil),
            Value::Bool(x) => Ok(Key::Bool(x)),
            Value::Number(x) => Ok(Key::NumberBits(x.to_bits() as i64)),
            Value::String(x) => Ok(Key::String(x)),
            Value::List(x) => Ok(Key::List(
                x.borrow()
                    .iter()
                    .map(Key::try_from)
                    .collect::<Result<Vec<_>>>()?,
            )),
            Value::Set(x) => Ok(Key::Set(HSet(match Rc::try_unwrap(x) {
                Ok(x) => x.into_inner(),
                Err(x) => x.borrow().clone(),
            }))),
            _ => Err(rterr!("{:?} could not be converted into a key", value)),
        }
    }
}

impl From<Key> for Value {
    fn from(key: Key) -> Self {
        match key {
            Key::Nil => Value::Nil,
            Key::Bool(x) => Value::Bool(x),
            Key::NumberBits(bits) => Value::Number(f64::from_bits(bits as u64)),
            Key::String(s) => Value::String(s),
            Key::List(list) => list
                .into_iter()
                .map(Value::from)
                .collect::<Vec<Value>>()
                .into(),
            Key::Set(set) => set.0.into(),
        }
    }
}

impl From<ConstVal> for Value {
    fn from(cv: ConstVal) -> Self {
        match cv {
            ConstVal::Nil => Value::Nil,
            ConstVal::Bool(x) => Value::Bool(x),
            ConstVal::Number(x) => Value::Number(x),
            ConstVal::String(x) => Value::String(x),
        }
    }
}

impl From<&ConstVal> for Value {
    fn from(cv: &ConstVal) -> Self {
        cv.clone().into()
    }
}

impl TryFrom<OsString> for RcStr {
    type Error = Error;

    fn try_from(s: OsString) -> Result<Self> {
        match s.into_string() {
            Ok(string) => Ok(string.into()),
            Err(osstr) => Err(rterr!("Got a string that is not valid UTF-8 ({:?})", osstr)),
        }
    }
}

impl TryFrom<&OsString> for RcStr {
    type Error = Error;

    fn try_from(s: &OsString) -> Result<Self> {
        RcStr::try_from(s.to_owned())
    }
}

impl TryFrom<&OsStr> for RcStr {
    type Error = Error;

    fn try_from(s: &OsStr) -> Result<Self> {
        RcStr::try_from(s.to_owned())
    }
}

macro_rules! try_from_for_int {
    ($t:ty) => {
        impl TryFrom<&Value> for $t {
            type Error = Error;

            fn try_from(v: &Value) -> Result<Self> {
                let x = v.number()?;
                if x < <$t>::MIN as f64
                    || x > <$t>::MAX as f64
                    || !x.is_finite()
                    || x.fract() != 0.0
                {
                    Err(rterr!(
                        concat!("Expected ", stringify!($t), " but got {:?}"),
                        x
                    ))
                } else {
                    Ok(x as $t)
                }
            }
        }
        impl TryFrom<Value> for $t {
            type Error = Error;

            fn try_from(v: Value) -> Result<Self> {
                <$t>::try_from(&v)
            }
        }
    };
}

try_from_for_int!(isize);
try_from_for_int!(i64);
try_from_for_int!(i32);
try_from_for_int!(i16);
try_from_for_int!(i8);
try_from_for_int!(usize);
try_from_for_int!(u64);
try_from_for_int!(u32);
try_from_for_int!(u16);
try_from_for_int!(u8);

impl From<Error> for Value {
    fn from(error: Error) -> Self {
        // For now, we just convert error objects to a pair
        // of strings
        Value::from(vec![
            Value::from(error.type_()),
            Value::from(error.message()),
        ])
    }
}

impl TryFrom<Value> for Error {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        if let Value::String(message) = value {
            Ok(Error::rt(message, vec![]))
        } else {
            let [type_, message] = value.unpack2_limited()?;
            let type_ = type_.into_string()?;
            let message = message.into_string()?;
            Ok(Error::new(type_, message, vec![]))
        }
    }
}
