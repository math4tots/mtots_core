use super::*;

impl Value {
    pub fn f64(&self) -> Result<f64> {
        self.number()
    }
    pub fn f32(&self) -> Result<f32> {
        Ok(self.number()? as f32)
    }
    pub fn f2usize(x: f64) -> Result<usize> {
        if x < 0.0 || x.fract() != 0.0 || !x.is_finite() || x > (usize::MAX as f64) {
            Err(rterr!("Expected usize, but got {:?}", x))
        } else {
            Ok(x as usize)
        }
    }
    pub fn usize(&self) -> Result<usize> {
        Self::f2usize(self.number()?)
    }
    pub fn f2u64(x: f64) -> Result<u64> {
        if x < 0.0 || x.fract() != 0.0 || !x.is_finite() {
            Err(rterr!("Expected u64, but got {:?}", x))
        } else {
            Ok(x as u64)
        }
    }
    pub fn u64(&self) -> Result<u64> {
        Self::f2u64(self.number()?)
    }
    pub fn f2u32(x: f64) -> Result<u32> {
        if x < 0.0 || x.fract() != 0.0 || !x.is_finite() || x > (u32::MAX as f64) {
            Err(rterr!("Expected u32, but got {:?}", x))
        } else {
            Ok(x as u32)
        }
    }
    pub fn u32(&self) -> Result<u32> {
        Self::f2u32(self.number()?)
    }
    pub fn f2u16(x: f64) -> Result<u16> {
        if x < 0.0 || x.fract() != 0.0 || !x.is_finite() || x > (u16::MAX as f64) {
            Err(rterr!("Expected u16, but got {:?}", x))
        } else {
            Ok(x as u16)
        }
    }
    pub fn u16(&self) -> Result<u16> {
        Self::f2u16(self.number()?)
    }
    pub fn f2u8(x: f64) -> Result<u8> {
        if x < 0.0 || x.fract() != 0.0 || !x.is_finite() || x > (u8::MAX as f64) {
            Err(rterr!("Expected u8, but got {:?}", x))
        } else {
            Ok(x as u8)
        }
    }
    pub fn u8(&self) -> Result<u8> {
        Self::f2u8(self.number()?)
    }
    pub fn f2isize(x: f64) -> Result<isize> {
        if x.fract() != 0.0 || !x.is_finite() || x < (isize::MIN as f64) || x > (isize::MAX as f64)
        {
            Err(rterr!("Expected isize, but got {:?}", x))
        } else {
            Ok(x as isize)
        }
    }
    pub fn isize(&self) -> Result<isize> {
        Self::f2isize(self.number()?)
    }
    pub fn f2i64(x: f64) -> Result<i64> {
        if x.fract() != 0.0 || !x.is_finite() {
            Err(rterr!("Expected i64, but got {:?}", x))
        } else {
            Ok(x as i64)
        }
    }
    pub fn i64(&self) -> Result<i64> {
        Self::f2i64(self.number()?)
    }
    pub fn f2i32(x: f64) -> Result<i32> {
        if x.fract() != 0.0 || !x.is_finite() || x < (i32::MIN as f64) || x > (i32::MAX as f64) {
            Err(rterr!("Expected i32, but got {:?}", x))
        } else {
            Ok(x as i32)
        }
    }
    pub fn i32(&self) -> Result<i32> {
        Self::f2i32(self.number()?)
    }
    pub fn f2i16(x: f64) -> Result<i16> {
        if x.fract() != 0.0 || !x.is_finite() || x < (i16::MIN as f64) || x > (i16::MAX as f64) {
            Err(rterr!("Expected i16, but got {:?}", x))
        } else {
            Ok(x as i16)
        }
    }
    pub fn i16(&self) -> Result<i16> {
        Self::f2i16(self.number()?)
    }
    pub fn f2i8(x: f64) -> Result<i8> {
        if x.fract() != 0.0 || !x.is_finite() || x < (i8::MIN as f64) || x > (i8::MAX as f64) {
            Err(rterr!("Expected i8, but got {:?}", x))
        } else {
            Ok(x as i8)
        }
    }
    pub fn i8(&self) -> Result<i8> {
        Self::f2i8(self.number()?)
    }
}
