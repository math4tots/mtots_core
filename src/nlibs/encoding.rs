use crate::Error;
use crate::Globals;
use crate::RcStr;
use crate::Result;
use crate::Value;
use std::convert::TryFrom;

pub enum Encoding {
    Raw,
    Utf8,
}

impl Encoding {
    pub fn decode(&self, globals: &mut Globals, bytes: Vec<u8>) -> Result<Value> {
        match self {
            Self::Raw => globals.new_handle(bytes).map(Value::from),
            Self::Utf8 => Ok(RcStr::from(std::str::from_utf8(&bytes)?).into()),
        }
    }
}

impl TryFrom<Value> for Encoding {
    type Error = Error;
    fn try_from(value: Value) -> Result<Encoding> {
        let opt = match &value {
            Value::Nil => Some(Encoding::Raw),
            Value::String(string) => match string.str() {
                "raw" => Some(Encoding::Raw),
                "utf8" | "utf-8" => Some(Encoding::Utf8),
                _ => None,
            },
            _ => None,
        };
        opt.ok_or_else(|| {
            rterr!(
                "Expected nil, 'raw', 'utf8', or 'utf-8' but got {:?}",
                value,
            )
        })
    }
}
