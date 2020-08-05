use std::borrow::Borrow;
use std::fmt;
use std::ops;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;

/// Implemented like Rc<PathBuf> so that it can stay a thin pointer,
/// but dereferences to &Path like Rc<Path>
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RcPath(Rc<PathBuf>);

impl RcPath {
    pub fn is_ptr_eq(&self, other: &Self) -> bool {
        fn ptr<T: ?Sized>(p: &Rc<T>) -> *const T {
            let p: &T = &*p;
            p as *const T
        }

        ptr(&self.0) == ptr(&other.0)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    pub fn try_unwrap(rcstr: Self) -> Result<PathBuf, RcPath> {
        match Rc::try_unwrap(rcstr.0) {
            Ok(s) => Ok(s),
            Err(rc) => Err(RcPath(rc)),
        }
    }

    pub fn unwrap_or_clone(rcstr: Self) -> PathBuf {
        match Self::try_unwrap(rcstr) {
            Ok(s) => s,
            Err(rcstr) => (*rcstr.0).clone(),
        }
    }
}

impl ops::Deref for RcPath {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.0
    }
}

impl AsRef<Path> for RcPath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl Borrow<Path> for RcPath {
    fn borrow(&self) -> &Path {
        &self.0
    }
}

impl fmt::Debug for RcPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl From<PathBuf> for RcPath {
    fn from(s: PathBuf) -> Self {
        RcPath(s.into())
    }
}

impl From<&Path> for RcPath {
    fn from(s: &Path) -> Self {
        RcPath(s.to_owned().into())
    }
}
