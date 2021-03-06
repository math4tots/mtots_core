use super::*;

/// Describes a parse time constant value
#[derive(Clone)]
pub enum ConstVal {
    Invalid,
    Nil,
    Bool(bool),
    Number(f64),
    String(RcStr),
    List(Vec<ConstVal>),
}

impl fmt::Display for ConstVal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(s) => write!(f, "{}", s),
            _ => write!(f, "{:?}", self),
        }
    }
}

impl fmt::Debug for ConstVal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid => write!(f, "<invalid>"),
            Self::Nil => write!(f, "nil"),
            Self::Bool(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            Self::Number(n) => write!(f, "{}", n),
            Self::String(s) => write!(f, "{:?}", s),
            Self::List(xs) => {
                write!(f, "[")?;
                let mut first = true;
                for x in xs {
                    if !first {
                        write!(f, ", ")?;
                    }
                    first = false;
                    write!(f, "{:?}", x)?;
                }
                write!(f, "]")
            }
        }
    }
}

impl From<()> for ConstVal {
    fn from(_: ()) -> Self {
        Self::Nil
    }
}

impl From<bool> for ConstVal {
    fn from(x: bool) -> Self {
        Self::Bool(x)
    }
}

impl From<i64> for ConstVal {
    fn from(x: i64) -> Self {
        Self::Number(x as f64)
    }
}

impl From<i32> for ConstVal {
    fn from(x: i32) -> Self {
        Self::Number(x as f64)
    }
}

impl From<i16> for ConstVal {
    fn from(x: i16) -> Self {
        Self::Number(x as f64)
    }
}

impl From<i8> for ConstVal {
    fn from(x: i8) -> Self {
        Self::Number(x as f64)
    }
}

impl From<u64> for ConstVal {
    fn from(x: u64) -> Self {
        Self::Number(x as f64)
    }
}

impl From<u32> for ConstVal {
    fn from(x: u32) -> Self {
        Self::Number(x as f64)
    }
}

impl From<u16> for ConstVal {
    fn from(x: u16) -> Self {
        Self::Number(x as f64)
    }
}

impl From<u8> for ConstVal {
    fn from(x: u8) -> Self {
        Self::Number(x as f64)
    }
}

impl From<usize> for ConstVal {
    fn from(x: usize) -> Self {
        Self::Number(x as f64)
    }
}

impl From<isize> for ConstVal {
    fn from(x: isize) -> Self {
        Self::Number(x as f64)
    }
}

impl From<f32> for ConstVal {
    fn from(x: f32) -> Self {
        Self::Number(x as f64)
    }
}

impl From<f64> for ConstVal {
    fn from(x: f64) -> Self {
        Self::Number(x)
    }
}

impl From<&str> for ConstVal {
    fn from(s: &str) -> Self {
        Self::String(s.into())
    }
}

impl From<&String> for ConstVal {
    fn from(s: &String) -> Self {
        Self::String(s.into())
    }
}

impl From<RcStr> for ConstVal {
    fn from(s: RcStr) -> Self {
        Self::String(s)
    }
}

impl From<&RcStr> for ConstVal {
    fn from(s: &RcStr) -> Self {
        Self::String(s.clone())
    }
}

impl From<[ConstVal; 0]> for ConstVal {
    fn from(_: [ConstVal; 0]) -> Self {
        Self::List(vec![])
    }
}

macro_rules! define_from_for_len {
    ($n:tt) => {
        impl<T> From<[T; $n]> for ConstVal
        where
            T: Clone,
            ConstVal: From<T>,
        {
            fn from(arr: [T; $n]) -> Self {
                let mut vec = Vec::new();
                for t in &arr {
                    vec.push(t.clone().into());
                }
                Self::List(vec)
            }
        }
    };
}

define_from_for_len!(1);
define_from_for_len!(2);
define_from_for_len!(3);
define_from_for_len!(4);
define_from_for_len!(5);
define_from_for_len!(6);
define_from_for_len!(7);
define_from_for_len!(8);
