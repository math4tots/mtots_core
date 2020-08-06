use crate::NativeModule;

pub(super) fn new() -> NativeModule {
    NativeModule::new("a.math", |_globals, builder| {
        builder
            .val("PI", "", std::f64::consts::PI)
            .val("TAU", "", std::f64::consts::PI * 2.0)
            .val("E", "", std::f64::consts::E)
            .build()
    })
}
