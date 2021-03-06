use super::*;

impl Globals {
    pub fn stash(&self) -> &Stash {
        &self.stash
    }
    pub fn stash_mut(&mut self) -> &mut Stash {
        &mut self.stash
    }
}

#[derive(Default)]
pub struct Stash {
    map: HashMap<TypeId, Box<dyn Any>>,
}

impl Stash {
    pub fn has<T: Any>(&self) -> bool {
        self.map.contains_key(&TypeId::of::<T>())
    }
    pub fn set<T: Any>(&mut self, t: T) -> Result<()> {
        match self.map.entry(TypeId::of::<T>()) {
            Entry::Vacant(entry) => {
                entry.insert(Box::new(Rc::new(RefCell::new(t))));
                Ok(())
            }
            Entry::Occupied(_) => Err(rterr!(
                "A value of {:?} is already stashed",
                std::any::type_name::<T>()
            )),
        }
    }
    pub fn get<T: Any>(&self) -> Result<Ref<T>> {
        Ok(self.get_rc_ref()?.borrow())
    }
    pub fn get_mut<T: Any>(&self) -> Result<RefMut<T>> {
        Ok(self.get_rc_ref()?.borrow_mut())
    }
    pub fn get_rc<T: Any>(&self) -> Result<Rc<RefCell<T>>> {
        self.get_rc_ref().map(Clone::clone)
    }
    fn get_rc_ref<T: Any>(&self) -> Result<&Rc<RefCell<T>>> {
        if let Some(rc) = self.map.get(&TypeId::of::<T>()) {
            let cell: &Rc<RefCell<T>> = rc.downcast_ref().unwrap();
            Ok(cell)
        } else {
            Err(rterr!(
                "Stash entry for {:?} not found",
                std::any::type_name::<T>()
            ))
        }
    }
    pub fn remove_rc<T: Any>(&mut self) -> Rc<RefCell<T>> {
        *self
            .map
            .remove(&TypeId::of::<T>())
            .unwrap()
            .downcast::<Rc<RefCell<T>>>()
            .unwrap()
    }
    pub fn remove<T: Any>(&mut self) -> T {
        let rc = self.remove_rc();
        match Rc::try_unwrap(rc) {
            Ok(t) => t.into_inner(),
            Err(_) => panic!(
                "There are still outstanding references to {:?}",
                std::any::type_name::<T>()
            ),
        }
    }
}
