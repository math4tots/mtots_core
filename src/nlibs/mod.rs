//! Builtin native modules and bindings

use crate::Globals;
mod fs;
mod math;

impl Globals {
    pub fn add_builtin_native_libraries(&mut self) {
        self.add(fs::new()).unwrap();
        self.add(math::new()).unwrap();
    }
}
