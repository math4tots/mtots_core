use super::*;

impl Globals {
    pub fn new_handle<T: Any>(&mut self, t: T) -> Result<Handle<T>> {
        match self.handle_class_map.get(&TypeId::of::<T>()) {
            Some(cls) => Ok(Handle::new(cls.clone(), t)),
            None => Err(rterr!(
                concat!(
                    "Tried to create a handle to a native instance of {:?}, ",
                    "but no class has been registered with the type"
                ),
                std::any::type_name::<T>()
            )),
        }
    }
    pub fn set_handle_class<T: Any>(&mut self, cls: Rc<Class>) -> Result<()> {
        match self.handle_class_map.entry(TypeId::of::<T>()) {
            Entry::Occupied(_) => Err(rterr!(
                "Tried to register a handle class for {} when one was already registered",
                std::any::type_name::<T>(),
            )),
            Entry::Vacant(entry) => {
                entry.insert(cls);
                Ok(())
            }
        }
    }
}
