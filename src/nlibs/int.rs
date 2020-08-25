use crate::ConvertValue;
use crate::Globals;
use crate::NativeModule;
use crate::Result;
use crate::Value;

const NAME: &'static str = "a.int";

pub(super) fn new() -> NativeModule {
    NativeModule::new(NAME, |m| {
        m.doc("Utilities for dealing with integers");
        m.class::<IntType, _>("IntType", |cls| {
            cls.ifunc("and_", ["a", "b"], "", |owner, _globals, args, _| {
                let mut args = args.into_iter();
                let a = args.next().unwrap().f64()?;
                let b = args.next().unwrap().f64()?;
                Ok(Value::from(owner.borrow().and(a, b)))
            });
            cls.ifunc("or_", ["a", "b"], "", |owner, _globals, args, _| {
                let mut args = args.into_iter();
                let a = args.next().unwrap().f64()?;
                let b = args.next().unwrap().f64()?;
                Ok(Value::from(owner.borrow().or(a, b)))
            });
            cls.ifunc("xor", ["a", "b"], "", |owner, _globals, args, _| {
                let mut args = args.into_iter();
                let a = args.next().unwrap().f64()?;
                let b = args.next().unwrap().f64()?;
                Ok(Value::from(owner.borrow().xor(a, b)))
            });
        });
        m.field("u8", "", |globals, _| {
            Ok(globals.new_handle(IntType::u8)?.into())
        });
        m.field("i8", "", |globals, _| {
            Ok(globals.new_handle(IntType::i8)?.into())
        });
        m.field("u16", "", |globals, _| {
            Ok(globals.new_handle(IntType::u16)?.into())
        });
        m.field("i16", "", |globals, _| {
            Ok(globals.new_handle(IntType::i16)?.into())
        });
        m.field("u32", "", |globals, _| {
            Ok(globals.new_handle(IntType::u32)?.into())
        });
        m.field("i32", "", |globals, _| {
            Ok(globals.new_handle(IntType::i32)?.into())
        });
    })
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntType {
    u8,
    i8,
    u16,
    i16,
    u32,
    i32,
}

impl ConvertValue for IntType {
    fn convert(_globals: &mut Globals, value: &Value) -> Result<Self> {
        match value.string()?.str() {
            "u8" => Ok(IntType::u8),
            "i8" => Ok(IntType::i8),
            "u16" => Ok(IntType::u16),
            "i16" => Ok(IntType::i16),
            "u32" => Ok(IntType::u32),
            "i32" => Ok(IntType::i32),
            s => Err(rterr!("Expected IntType, but got {:?}", s)),
        }
    }
}

impl IntType {
    pub fn nbits(&self) -> usize {
        match self {
            Self::u8 => 8,
            Self::i8 => 8,
            Self::u16 => 16,
            Self::i16 => 16,
            Self::u32 => 32,
            Self::i32 => 32,
        }
    }
    pub fn nbytes(&self) -> usize {
        self.nbits() / 8
    }
    pub fn from_le_slice(&self, x: &[u8]) -> Result<f64> {
        if self.nbytes() != x.len() {
            return Err(rterr!(
                "{:?} requires {} bytes but got {}",
                self,
                self.nbytes(),
                x.len()
            ));
        }
        match self {
            Self::u8 => Ok(u8::from_le_bytes([x[0]]) as f64),
            Self::i8 => Ok(i8::from_le_bytes([x[0]]) as f64),
            Self::i16 => Ok(i16::from_le_bytes([x[0], x[1]]) as f64),
            Self::u16 => Ok(u16::from_le_bytes([x[0], x[1]]) as f64),
            Self::i32 => Ok(i32::from_le_bytes([x[0], x[1], x[2], x[3]]) as f64),
            Self::u32 => Ok(u32::from_le_bytes([x[0], x[1], x[2], x[3]]) as f64),
        }
    }
    pub fn from_be_slice(&self, x: &[u8]) -> Result<f64> {
        if self.nbytes() != x.len() {
            return Err(rterr!(
                "{:?} requires {} bytes but got {}",
                self,
                self.nbytes(),
                x.len()
            ));
        }
        match self {
            Self::u8 => Ok(u8::from_be_bytes([x[0]]) as f64),
            Self::i8 => Ok(i8::from_be_bytes([x[0]]) as f64),
            Self::i16 => Ok(i16::from_be_bytes([x[0], x[1]]) as f64),
            Self::u16 => Ok(u16::from_be_bytes([x[0], x[1]]) as f64),
            Self::i32 => Ok(i32::from_be_bytes([x[0], x[1], x[2], x[3]]) as f64),
            Self::u32 => Ok(u32::from_be_bytes([x[0], x[1], x[2], x[3]]) as f64),
        }
    }
    pub fn and(&self, a: f64, b: f64) -> f64 {
        match self {
            Self::u8 => ((a as u8) & (b as u8)) as f64,
            Self::i8 => ((a as i8) & (b as i8)) as f64,
            Self::u16 => ((a as u16) & (b as u16)) as f64,
            Self::i16 => ((a as i16) & (b as i16)) as f64,
            Self::u32 => ((a as u32) & (b as u32)) as f64,
            Self::i32 => ((a as i32) & (b as i32)) as f64,
        }
    }
    pub fn or(&self, a: f64, b: f64) -> f64 {
        match self {
            Self::u8 => ((a as u8) | (b as u8)) as f64,
            Self::i8 => ((a as i8) | (b as i8)) as f64,
            Self::u16 => ((a as u16) | (b as u16)) as f64,
            Self::i16 => ((a as i16) | (b as i16)) as f64,
            Self::u32 => ((a as u32) | (b as u32)) as f64,
            Self::i32 => ((a as i32) | (b as i32)) as f64,
        }
    }
    pub fn xor(&self, a: f64, b: f64) -> f64 {
        match self {
            Self::u8 => ((a as u8) ^ (b as u8)) as f64,
            Self::i8 => ((a as i8) ^ (b as i8)) as f64,
            Self::u16 => ((a as u16) ^ (b as u16)) as f64,
            Self::i16 => ((a as i16) ^ (b as i16)) as f64,
            Self::u32 => ((a as u32) ^ (b as u32)) as f64,
            Self::i32 => ((a as i32) ^ (b as i32)) as f64,
        }
    }
}
