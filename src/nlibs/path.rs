use crate::NativeModule;

const NAME: &'static str = "a.path";

pub(super) fn new() -> NativeModule {
    NativeModule::new(NAME, |builder| {
        builder.doc(concat!("Utility for dealing with filesystem paths",));
    })
}
