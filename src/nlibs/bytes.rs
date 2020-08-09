use crate::NativeModule;

const NAME: &'static str = "a.bytes";

pub(super) fn new() -> NativeModule {
    NativeModule::new(NAME, |builder| {
        builder
            .doc("Utilities for dealing with raw bytes")
            .class::<Vec<u8>, _>("Bytes", |_cls| {});
    })
}
