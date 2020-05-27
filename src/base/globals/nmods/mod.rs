/// Native modules
use super::Globals;

mod os;
mod osproc;

impl Globals {
    pub(super) fn add_builtin_native_modules(&mut self) {
        self.add_native_module(os::NAME.into(), os::load);
        self.add_native_module(osproc::NAME.into(), osproc::load);
    }
}