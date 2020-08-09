use super::*;

impl Value {
    pub fn unpack_into_set(self, globals: &mut Globals) -> Result<IndexSet<Key>> {
        match self {
            Self::Set(set) => match Rc::try_unwrap(set) {
                Ok(set) => Ok(set.into_inner()),
                Err(set) => Ok(set.borrow().clone()),
            },
            _ => self.unpack_into(globals),
        }
    }
    pub fn unpack_into_map(self, globals: &mut Globals) -> Result<IndexMap<Key, Value>> {
        match self {
            Self::Map(map) => match Rc::try_unwrap(map) {
                Ok(map) => Ok(map.into_inner()),
                Err(map) => Ok(map.borrow().clone()),
            },
            _ => self.unpack_into(globals),
        }
    }
    pub fn unpack(self, globals: &mut Globals) -> Result<Vec<Value>> {
        match self {
            Self::List(list) => match Rc::try_unwrap(list) {
                Ok(list) => Ok(list.into_inner()),
                Err(list) => Ok(list.borrow().clone()),
            },
            _ => self.unpack_into(globals),
        }
    }
    pub fn unpack_into<C, T, E>(self, globals: &mut Globals) -> Result<C>
    where
        C: FromIterator<T>,
        T: TryFrom<Value, Error = E>,
        Error: From<E>,
    {
        match self {
            Self::List(list) => match Rc::try_unwrap(list) {
                Ok(list) => Ok(list
                    .into_inner()
                    .into_iter()
                    .map(T::try_from)
                    .collect::<std::result::Result<_, _>>()?),
                Err(list) => Ok(list
                    .borrow()
                    .iter()
                    .map(Clone::clone)
                    .map(T::try_from)
                    .collect::<std::result::Result<_, _>>()?),
            },
            Self::Set(set) => match Rc::try_unwrap(set) {
                Ok(set) => Ok(set
                    .into_inner()
                    .into_iter()
                    .map(Value::from)
                    .map(T::try_from)
                    .collect::<std::result::Result<_, _>>()?),
                Err(set) => Ok(set
                    .borrow()
                    .iter()
                    .map(Value::from)
                    .map(T::try_from)
                    .collect::<std::result::Result<_, _>>()?),
            },
            Self::Map(map) => match Rc::try_unwrap(map) {
                Ok(map) => Ok(map
                    .into_inner()
                    .into_iter()
                    .map(Value::from)
                    .map(T::try_from)
                    .collect::<std::result::Result<_, _>>()?),
                Err(map) => Ok(map
                    .borrow()
                    .iter()
                    .map(Value::from)
                    .map(T::try_from)
                    .collect::<std::result::Result<_, _>>()?),
            },
            Self::Generator(gen) => Ok(gen
                .borrow_mut()
                .iter(globals)
                .map(|r| match r {
                    Ok(v) => Ok(T::try_from(v)?),
                    Err(error) => Err(error),
                })
                .collect::<std::result::Result<_, _>>()?),
            Self::NativeGenerator(gen) => Ok(gen
                .borrow_mut()
                .iter(globals)
                .map(|r| match r {
                    Ok(v) => Ok(T::try_from(v)?),
                    Err(error) => Err(error),
                })
                .collect::<std::result::Result<_, _>>()?),
            _ => Err(rterr!("{:?} is not unpackable", self)),
        }
    }
    pub fn unpack_limited(self) -> Result<Vec<Value>> {
        match self {
            Self::List(list) => match Rc::try_unwrap(list) {
                Ok(list) => Ok(list.into_inner()),
                Err(list) => Ok(list.borrow().clone()),
            },
            Self::Set(set) => match Rc::try_unwrap(set) {
                Ok(set) => Ok(set.into_inner().into_iter().map(Value::from).collect()),
                Err(set) => Ok(set.borrow().iter().map(Value::from).collect()),
            },
            Self::Map(map) => match Rc::try_unwrap(map) {
                Ok(map) => Ok(map.into_inner().into_iter().map(Value::from).collect()),
                Err(map) => Ok(map.borrow().iter().map(Value::from).collect()),
            },
            _ => Err(rterr!("{:?} is not unpackable in this context", self)),
        }
    }
    pub fn unpack2_limited(self) -> Result<[Value; 2]> {
        let vec = self.unpack_limited()?;
        if vec.len() != 2 {
            Err(rterr!("Expected {} elements but got {}", 2, vec.len()))
        } else {
            let mut iter = vec.into_iter();
            Ok([iter.next().unwrap(), iter.next().unwrap()])
        }
    }
    pub fn unpack3_limited(self) -> Result<[Value; 3]> {
        let vec = self.unpack_limited()?;
        if vec.len() != 3 {
            Err(rterr!("Expected {} elements but got {}", 3, vec.len()))
        } else {
            let mut iter = vec.into_iter();
            Ok([
                iter.next().unwrap(),
                iter.next().unwrap(),
                iter.next().unwrap(),
            ])
        }
    }
    pub fn unpack4_limited(self) -> Result<[Value; 4]> {
        let vec = self.unpack_limited()?;
        if vec.len() != 4 {
            Err(rterr!("Expected {} elements but got {}", 4, vec.len()))
        } else {
            let mut iter = vec.into_iter();
            Ok([
                iter.next().unwrap(),
                iter.next().unwrap(),
                iter.next().unwrap(),
                iter.next().unwrap(),
            ])
        }
    }
    pub fn unpack_keyval(self, globals: &mut Globals) -> Result<(Key, Value)> {
        let [key, val] = self.unpack2(globals)?;
        let key = Key::try_from(key)?;
        Ok((key, val))
    }
    pub fn unpack2(self, globals: &mut Globals) -> Result<[Value; 2]> {
        let vec = self.unpack(globals)?;
        if vec.len() != 2 {
            Err(rterr!("Expected {} elements but got {}", 2, vec.len()))
        } else {
            let mut iter = vec.into_iter();
            Ok([iter.next().unwrap(), iter.next().unwrap()])
        }
    }
    pub fn unpack3(self, globals: &mut Globals) -> Result<[Value; 3]> {
        let vec = self.unpack(globals)?;
        if vec.len() != 3 {
            Err(rterr!("Expected {} elements but got {}", 3, vec.len()))
        } else {
            let mut iter = vec.into_iter();
            Ok([
                iter.next().unwrap(),
                iter.next().unwrap(),
                iter.next().unwrap(),
            ])
        }
    }
    pub fn unpack4(self, globals: &mut Globals) -> Result<[Value; 4]> {
        let vec = self.unpack(globals)?;
        if vec.len() != 4 {
            Err(rterr!("Expected {} elements but got {}", 4, vec.len()))
        } else {
            let mut iter = vec.into_iter();
            Ok([
                iter.next().unwrap(),
                iter.next().unwrap(),
                iter.next().unwrap(),
                iter.next().unwrap(),
            ])
        }
    }
    pub fn unpack2_f32(self, globals: &mut Globals) -> Result<[f32; 2]> {
        let [x1, x2] = self.unpack2(globals)?;
        let x1 = x1.f32()?;
        let x2 = x2.f32()?;
        Ok([x1, x2])
    }
    pub fn unpack3_f32(self, globals: &mut Globals) -> Result<[f32; 3]> {
        let [x1, x2, x3] = self.unpack3(globals)?;
        let x1 = x1.f32()?;
        let x2 = x2.f32()?;
        let x3 = x3.f32()?;
        Ok([x1, x2, x3])
    }
    pub fn unpack4_f32(self, globals: &mut Globals) -> Result<[f32; 4]> {
        let [x1, x2, x3, x4] = self.unpack4(globals)?;
        let x1 = x1.f32()?;
        let x2 = x2.f32()?;
        let x3 = x3.f32()?;
        let x4 = x4.f32()?;
        Ok([x1, x2, x3, x4])
    }
}
