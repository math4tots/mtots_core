//! Builtin native modules and bindings

use crate::Globals;
mod bytes;
mod encoding;
mod env;
mod fs;
mod int;
mod math;
mod os;
mod procc;
mod sys;
mod time;

pub use encoding::Encoding;
pub use int::IntType;

impl Globals {
    pub fn add_builtin_native_libraries(&mut self) {
        self.add_native_module(bytes::new()).unwrap();
        self.add_native_module(env::new()).unwrap();
        self.add_native_module(fs::new()).unwrap();
        self.add_native_module(int::new()).unwrap();
        self.add_native_module(math::new()).unwrap();
        self.add_native_module(os::new()).unwrap();
        self.add_native_module(procc::new()).unwrap();
        self.add_native_module(sys::new()).unwrap();
        self.add_native_module(time::new()).unwrap();
    }
}
