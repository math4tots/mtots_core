use crate::NativeModule;

pub(super) fn new() -> NativeModule {
    NativeModule::builder("a.fs").build()
}
