use crate::NativeModule;

pub(super) fn new() -> NativeModule {
    NativeModule::new("a.math", |_globals, builder| {
        builder
            .val("PI", None, std::f64::consts::PI)
            .val("TAU", None, std::f64::consts::PI * 2.0)
            .val("E", None, std::f64::consts::E)
            .build()
    })
}
