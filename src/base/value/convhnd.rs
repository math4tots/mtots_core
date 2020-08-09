use super::*;

/// Trait for values that can imply a native Rust type
/// e.g. a String can be provided wherever a Path is needed
pub trait ConvertToHandle: Any + Sized {
    fn convert(globals: &mut Globals, value: Value) -> Result<Handle<Self>>;
}
