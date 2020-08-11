use std::borrow::Borrow;
use std::cell::RefMut;
use std::fmt;
use std::ops::Deref;

/// Kind of like Cow, but might also be a RefMut (from a RefCell)
/// Also, does not use ToOwned.
/// This is because the main point of an XRefMut is not to Copy On Write,
/// but to either hold an owned value or a reference to a value
pub enum XRefMut<'a, T> {
    Borrowed(&'a mut T),
    Ref(RefMut<'a, T>),
    Owned(T),
}

impl<'a, T> XRefMut<'a, T> {
    pub fn to_ref(&self) -> &T {
        match self {
            Self::Borrowed(r) => r,
            Self::Ref(r) => r,
            Self::Owned(t) => t,
        }
    }
    pub fn to_mut(&mut self) -> &mut T {
        match self {
            Self::Borrowed(r) => r,
            Self::Ref(r) => r,
            Self::Owned(ref mut t) => t,
        }
    }
}

impl<'a, T: Clone> XRefMut<'a, T> {
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

impl<'a, T> AsRef<T> for XRefMut<'a, T> {
    fn as_ref(&self) -> &T {
        match self {
            Self::Borrowed(r) => r,
            Self::Ref(r) => r,
            Self::Owned(t) => t.borrow(),
        }
    }
}

impl<'a, T> Borrow<T> for XRefMut<'a, T> {
    fn borrow(&self) -> &T {
        &**self
    }
}

impl<'a, T: fmt::Debug> fmt::Debug for XRefMut<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Borrowed(r) => fmt::Debug::fmt(r, f),
            Self::Ref(r) => fmt::Debug::fmt(r, f),
            Self::Owned(t) => fmt::Debug::fmt(t, f),
        }
    }
}

impl<'a, T: fmt::Display> fmt::Display for XRefMut<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Borrowed(r) => fmt::Display::fmt(r, f),
            Self::Ref(r) => fmt::Display::fmt(r, f),
            Self::Owned(t) => fmt::Display::fmt(t, f),
        }
    }
}

impl<'a, T: Default> Default for XRefMut<'a, T> {
    fn default() -> Self {
        Self::Owned(Default::default())
    }
}

impl<'a, T> Deref for XRefMut<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        match self {
            Self::Borrowed(r) => r,
            Self::Ref(r) => r,
            Self::Owned(t) => t.borrow(),
        }
    }
}
