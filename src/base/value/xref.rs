use crate::Globals;
use crate::Result;
use crate::Value;
use crate::XRef;
use crate::XRefMut;
use std::any::Any;
use std::cell::Ref;
use std::cell::RefMut;

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
    pub fn to_xref_mut<T: Any + ConvertValue>(&self, globals: &mut Globals) -> Result<XRefMut<T>> {
        if self.is_handle::<T>() {
            Ok(XRefMut::Ref(self.borrow_mut_handle_or_panic()))
        } else {
            Ok(XRefMut::Owned(T::convert(globals, self)?))
        }
    }
    pub(crate) fn borrow_handle_or_panic<T: Any>(&self) -> Ref<T> {
        if let Self::Handle(data) = self {
            data.borrow()
        } else {
            panic!("borrow_handle_or_panic: type mismatch")
        }
    }
    pub(crate) fn borrow_mut_handle_or_panic<T: Any>(&self) -> RefMut<T> {
        if let Self::Handle(data) = self {
            data.borrow_mut()
        } else {
            panic!("borrow_mut_handle_or_panic: type mismatch")
        }
    }
}

/// Trait to indicate that a value may be converted into a value of the given type.
pub trait ConvertValue: Sized + 'static {
    fn convert(globals: &mut Globals, _value: &Value) -> Result<Self> {
        // Default implementation will just always fail.
        // This is useful in cases where, we don't have a automatic conversion
        // method, and we really just want Value::convert to just check whether
        // it is a Handle
        Err(match globals.get_handle_class::<Self>() {
            Some(cls) => rterr!("Expected {} value", cls.name()),
            None => rterr!("Expected {:?} native handle", std::any::type_name::<Self>()),
        })
    }
}
