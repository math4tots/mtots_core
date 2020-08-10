use std::borrow;
use std::cmp;
use std::ffi::OsStr;
use std::fmt;
use std::hash;
use std::ops;
use std::rc::Rc;

/// str smarat pointer that also stashes chars as
/// needed so that char access can be constant time
#[derive(Clone)]
pub struct RcStr(Rc<String>);

impl RcStr {
    pub fn unwrap_or_clone(self) -> String {
        match Rc::try_unwrap(self.0) {
            Ok(string) => string,
            Err(string) => (*string).clone(),
        }
    }
    pub fn str(&self) -> &str {
        &*self.0
    }
}

impl fmt::Debug for RcStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", &*self.0)
    }
}

impl fmt::Display for RcStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &*self.0)
    }
}

impl borrow::Borrow<str> for RcStr {
    fn borrow(&self) -> &str {
        self.str()
    }
}

impl ops::Deref for RcStr {
    type Target = str;

    fn deref(&self) -> &str {
        self.str()
    }
}

impl AsRef<[u8]> for RcStr {
    fn as_ref(&self) -> &[u8] {
        self.str().as_ref()
    }
}

impl AsRef<str> for RcStr {
    fn as_ref(&self) -> &str {
        self.str()
    }
}

impl AsRef<OsStr> for RcStr {
    fn as_ref(&self) -> &OsStr {
        self.str().as_ref()
    }
}

impl cmp::PartialOrd for RcStr {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl cmp::Ord for RcStr {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl cmp::PartialEq for RcStr {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl cmp::Eq for RcStr {}

impl hash::Hash for RcStr {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl ops::Index<ops::Range<usize>> for RcStr {
    type Output = str;
    fn index(&self, index: ops::Range<usize>) -> &str {
        self.0.index(index)
    }
}

impl ops::Index<ops::RangeTo<usize>> for RcStr {
    type Output = str;
    fn index(&self, index: ops::RangeTo<usize>) -> &str {
        self.0.index(index)
    }
}

impl ops::Index<ops::RangeFrom<usize>> for RcStr {
    type Output = str;
    fn index(&self, index: ops::RangeFrom<usize>) -> &str {
        self.0.index(index)
    }
}

impl ops::Index<ops::RangeFull> for RcStr {
    type Output = str;
    fn index(&self, index: ops::RangeFull) -> &str {
        self.0.index(index)
    }
}

impl From<String> for RcStr {
    fn from(string: String) -> Self {
        Self(Rc::new(string))
    }
}

impl From<&String> for RcStr {
    fn from(string: &String) -> Self {
        string.clone().into()
    }
}

impl From<&str> for RcStr {
    fn from(s: &str) -> Self {
        s.to_owned().into()
    }
}

impl From<&&str> for RcStr {
    fn from(s: &&str) -> Self {
        s.to_owned().into()
    }
}
