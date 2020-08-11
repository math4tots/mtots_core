use std::borrow::Borrow;
use std::cell::Ref;
use std::fmt;
use std::ops::Deref;

/// Kind of like Cow, but might also be a Ref (from a RefCell)
/// Also, does not use ToOwned.
/// This is because the main point of an XRef is not to Copy On Write,
/// but to either hold an owned value or a reference to a value
pub enum XRef<'a, T> {
    Borrowed(&'a T),
    Ref(Ref<'a, T>),
    Owned(T),
}

impl<'a, T> XRef<'a, T> {
    pub fn to_ref(&self) -> &T {
        match self {
            Self::Borrowed(r) => r,
            Self::Ref(r) => r,
            Self::Owned(t) => &t,
        }
    }
}

impl<'a, T: Clone> XRef<'a, T> {
    /// Acquires a mutable reference to the owned form of the data.
    /// Clones the data if it is not already owned.
    pub fn to_mut(&mut self) -> &mut T {
        match self {
            Self::Borrowed(r) => {
                *self = Self::Owned(r.clone());
                match *self {
                    Self::Borrowed(..) | Self::Ref(..) => unreachable!(),
                    Self::Owned(ref mut owned) => owned,
                }
            }
            Self::Ref(r) => {
                *self = Self::Owned(r.clone());
                match *self {
                    Self::Borrowed(..) | Self::Ref(..) => unreachable!(),
                    Self::Owned(ref mut owned) => owned,
                }
            }
            Self::Owned(ref mut t) => t,
        }
    }
    /// Extracts the owned data.
    /// Clones the data if it is not already owned.
    pub fn into_owned(self) -> T {
        match self {
            Self::Borrowed(r) => r.clone(),
            Self::Ref(r) => r.clone(),
            Self::Owned(t) => t,
        }
    }
}

impl<'a, T> AsRef<T> for XRef<'a, T> {
    fn as_ref(&self) -> &T {
        match self {
            Self::Borrowed(r) => r,
            Self::Ref(r) => r,
            Self::Owned(t) => t.borrow(),
        }
    }
}

impl<'a, T> Borrow<T> for XRef<'a, T> {
    fn borrow(&self) -> &T {
        &**self
    }
}

impl<'a, T: Clone> Clone for XRef<'a, T> {
    fn clone(&self) -> Self {
        match self {
            Self::Borrowed(r) => XRef::Borrowed(r),
            Self::Ref(r) => XRef::Ref(Ref::clone(r)),
            Self::Owned(t) => Self::Owned(t.clone()),
        }
    }
}

impl<'a, T: fmt::Debug> fmt::Debug for XRef<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Borrowed(r) => fmt::Debug::fmt(r, f),
            Self::Ref(r) => fmt::Debug::fmt(r, f),
            Self::Owned(t) => fmt::Debug::fmt(t, f),
        }
    }
}

impl<'a, T: fmt::Display> fmt::Display for XRef<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Borrowed(r) => fmt::Display::fmt(r, f),
            Self::Ref(r) => fmt::Display::fmt(r, f),
            Self::Owned(t) => fmt::Display::fmt(t, f),
        }
    }
}

impl<'a, T: Default> Default for XRef<'a, T> {
    fn default() -> Self {
        Self::Owned(Default::default())
    }
}

impl<'a, T> Deref for XRef<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        match self {
            Self::Borrowed(r) => r,
            Self::Ref(r) => r,
            Self::Owned(t) => t.borrow(),
        }
    }
}
