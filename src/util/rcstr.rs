use std::borrow::Borrow;
use std::fmt;
use std::ops;
use std::rc::Rc;

/// Implemented like Rc<String> so that it can stay a thin pointer,
/// but dereferences to &str like Rc<str>
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RcStr(Rc<String>);

impl RcStr {
    pub fn str(&self) -> &str {
        &self.0
    }

    pub fn try_unwrap(rcstr: Self) -> Result<String, RcStr> {
        match Rc::try_unwrap(rcstr.0) {
            Ok(s) => Ok(s),
            Err(rc) => Err(RcStr(rc)),
        }
    }

    pub fn unwrap_or_clone(rcstr: Self) -> String {
        match Self::try_unwrap(rcstr) {
            Ok(s) => s,
            Err(rcstr) => (*rcstr.0).clone(),
        }
    }
}

impl ops::Deref for RcStr {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for RcStr {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for RcStr {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for RcStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for RcStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl From<String> for RcStr {
    fn from(s: String) -> Self {
        RcStr(s.into())
    }
}

impl From<&str> for RcStr {
    fn from(s: &str) -> Self {
        RcStr(s.to_owned().into())
    }
}
