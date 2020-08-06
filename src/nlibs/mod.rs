use crate::Class;
use crate::Globals;
use crate::NativeModule;

impl Globals {
    pub fn add_builtin_native_libraries(&mut self) {
        self.add(NativeModule::builder("a._os").field("Path", |_globals| {
            Ok(Class::new(
                "Path".into(),
                Class::map_from_funcs(vec![]),
                Class::map_from_funcs(vec![]),
            )
            .into())
        }))
        .unwrap();
    }
}
