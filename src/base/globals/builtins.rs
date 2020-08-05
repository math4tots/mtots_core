use super::Globals;
use crate::BuiltinClasses;
use crate::BuiltinExceptions;
use crate::NativeFunctions;
use crate::RcStr;
use crate::Value;
use std::collections::HashMap;

impl Globals {
    pub(super) fn new_builtins(
        bclss: &BuiltinClasses,
        bfuncs: &NativeFunctions,
        bexcs: &BuiltinExceptions,
    ) -> HashMap<RcStr, Value> {
        let mut map = HashMap::new();
        for cls in bclss.list() {
            map.insert(cls.short_name().clone(), Value::Class(cls.clone()));
        }
        for exc in bexcs.for_builtins() {
            map.insert(exc.name().clone(), Value::ExceptionKind(exc.clone()));
        }
        for func in bfuncs.for_builtins() {
            map.insert(func.name().clone(), Value::NativeFunction(func.clone()));
        }
        map
    }
}
