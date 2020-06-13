/// Native modules
use super::Globals;

mod math;
mod mfs;
mod os;
mod osproc;
mod time;

impl Globals {
    pub(super) fn add_builtin_native_modules(&mut self) {
        self.add_native_module(os::NAME.into(), os::load);
        self.add_native_module(osproc::NAME.into(), osproc::load);
        self.add_native_module(time::NAME.into(), time::load);
        self.add_native_module(math::NAME.into(), math::load);
        self.add_native_module(mfs::NAME.into(), mfs::load);
    }
}
