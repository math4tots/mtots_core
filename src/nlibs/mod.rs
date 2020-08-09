//! Builtin native modules and bindings

use crate::Globals;
mod bytes;
mod env;
mod fs;
mod math;
mod os;
mod path;

impl Globals {
    pub fn add_builtin_native_libraries(&mut self) {
        self.add(bytes::new()).unwrap();
        self.add(env::new()).unwrap();
        self.add(fs::new()).unwrap();
        self.add(math::new()).unwrap();
        self.add(os::new()).unwrap();
        self.add(path::new()).unwrap();
    }
}
