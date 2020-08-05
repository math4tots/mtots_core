use super::*;

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
