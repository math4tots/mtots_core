use super::*;

/// Describes a parse time constant value
#[derive(Debug, Clone)]
pub enum ConstVal {
    Nil,
    Bool(bool),
    Number(f64),
    String(RcStr),
}

impl fmt::Display for ConstVal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nil => write!(f, "nil"),
            Self::Bool(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            Self::Number(n) => write!(f, "{}", n),
            Self::String(s) => write!(f, "{:?}", s),
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
