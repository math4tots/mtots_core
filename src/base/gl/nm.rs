use super::*;

#[derive(Clone)]
pub struct NativeModule {
    pub name: RcStr,
    pub fields: Vec<RcStr>,
    pub init: fn(&mut Globals, &HashMap<RcStr, Rc<RefCell<Value>>>) -> Result<()>,
}
