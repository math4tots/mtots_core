use crate::ConvertIntoHandle;
use crate::Globals;
use crate::Handle;
use crate::NativeModule;
use crate::Result;
use crate::Value;
use std::path::PathBuf;

const NAME: &'static str = "a.path";

impl ConvertIntoHandle for PathBuf {
    fn convert(globals: &mut Globals, value: Value) -> Result<Handle<PathBuf>> {
        let path = PathBuf::from(value.into_string()?.str());
        globals.new_handle(path)
    }
}

pub(super) fn new() -> NativeModule {
    NativeModule::new(NAME, |m| {
        m.doc(concat!("Utility for dealing with filesystem paths"));

        m.class::<PathBuf, _>("Path", |cls| {
            cls.sfunc(
                "__call",
                ["path"],
                concat!(
                    "Converts a string into a Path\n",
                    "If a Path is given, it will return the value as is\n",
                ),
                |globals, args, _| {
                    Ok(args
                        .into_iter()
                        .next()
                        .unwrap()
                        .convert_to_handle::<PathBuf>(globals)?
                        .into())
                },
            );
            cls.ifunc("parent", (), None, |owner, globals, _, _| {
                owner
                    .borrow()
                    .parent()
                    .map(PathBuf::from)
                    .map(|p| globals.new_handle(p).map(Value::from))
                    .unwrap_or(Ok(Value::Nil))
            });
        });
    })
}
