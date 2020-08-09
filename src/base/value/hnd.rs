use super::*;

pub struct HandleData {
    cls: Rc<Class>,
    typename: &'static str,
    data: RefCell<Box<dyn Any>>,
}

impl HandleData {
    pub fn cls(&self) -> &Rc<Class> {
        &self.cls
    }
    pub fn typename(&self) -> &'static str {
        self.typename
    }
    pub fn is<T: Any>(&self) -> bool {
        self.data.borrow().is::<T>()
    }
    pub fn downcast<T: Any>(data: Rc<HandleData>) -> Result<Handle<T>> {
        if data.is::<T>() {
            Ok(Handle(data, PhantomData))
        } else {
            Err(rterr!(
                "Expected {}/handle value, but got {}/handle value",
                std::any::type_name::<T>(),
                data.typename
            ))
        }
    }
}

impl cmp::PartialEq for HandleData {
    fn eq(&self, other: &Self) -> bool {
        (self as *const Self).eq(&(other as *const Self))
    }
}

impl cmp::PartialOrd for HandleData {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        (self as *const Self).partial_cmp(&(other as *const Self))
    }
}

impl fmt::Debug for HandleData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{}/{} native value>", self.cls.name(), self.typename())
    }
}

pub struct Handle<T: Any>(Rc<HandleData>, PhantomData<T>);

impl<T: Any> Handle<T> {
    pub(crate) fn new(cls: Rc<Class>, data: T) -> Self {
        Self(
            Rc::new(HandleData {
                cls,
                typename: std::any::type_name::<T>(),
                data: RefCell::new(Box::new(data)),
            }),
            PhantomData,
        )
    }
    pub fn borrow(&self) -> Ref<T> {
        Ref::map(self.0.data.borrow(), |r| r.downcast_ref().unwrap())
    }
    pub fn borrow_mut(&self) -> RefMut<T> {
        RefMut::map(self.0.data.borrow_mut(), |r| r.downcast_mut().unwrap())
    }
    /// Tries to take ownership of the underlying T value.
    /// Returns Ok(T) on success,
    /// Returns Err(Self) on failure
    ///     A failure may happen if there are still other references to the data
    pub fn try_unwrap(self) -> std::result::Result<T, Self> {
        match Rc::try_unwrap(self.0) {
            Ok(data) => Ok(*data.data.into_inner().downcast().unwrap()),
            Err(ptr) => Err(Self(ptr, PhantomData)),
        }
    }
    pub fn unwrap(self) -> Result<T> {
        match self.try_unwrap() {
            Ok(t) => Ok(t),
            Err(handle) => Err(rterr!(
                "Handle could not be unwraped into {} due to additional references",
                handle.0.typename()
            )),
        }
    }
}

impl<T: Any + Clone> Handle<T> {
    pub fn unwrap_or_clone(self) -> T {
        match self.try_unwrap() {
            Ok(t) => t,
            Err(handle) => handle.borrow().clone(),
        }
    }
}

impl<T: Any> From<Handle<T>> for Value {
    fn from(handle: Handle<T>) -> Self {
        Self::Handle(handle.0)
    }
}
