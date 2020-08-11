use crate::Globals;
use crate::XRef;
use crate::Result;
use crate::Value;
use std::any::Any;
use std::cell::Ref;

impl Value {
    /// For converting to a native value into a value of the desired type.
    /// If the current value is a handle of the given type, it will either
    /// unwrap or clone it.
    /// If not, it will try to convert it with the ConvertValue trait
    pub fn convert<T: Any + ConvertValue + Clone>(self, globals: &mut Globals) -> Result<T> {
        if self.is_handle::<T>() {
            Ok(self.into_handle::<T>()?.unwrap_or_clone())
        } else {
            T::convert(globals, &self)
        }
    }
    pub fn to_xref<T: Any + ConvertValue>(&self, globals: &mut Globals) -> Result<XRef<T>> {
        if self.is_handle::<T>() {
            Ok(XRef::Ref(self.borrow_handle_or_panic()))
        } else {
            Ok(XRef::Owned(T::convert(globals, self)?))
        }
    }
    pub(crate) fn borrow_handle_or_panic<T: Any>(&self) -> Ref<T> {
        if let Self::Handle(data) = self {
            data.borrow()
        } else {
            panic!("borrow_handle_or_panic: type mismatch")
        }
    }
}

/// Trait to indicate that a value may be converted into the given value.
pub trait ConvertValue: Sized {
    fn convert(globals: &mut Globals, value: &Value) -> Result<Self>;
}
