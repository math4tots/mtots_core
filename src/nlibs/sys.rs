use crate::NativeModule;
use crate::Value;
use crate::Source;

const NAME: &'static str = "a.sys";

pub(super) fn new() -> NativeModule {
    NativeModule::new(NAME, |m| {
        m.doc("Utilities for interacting with the interpreter");
        m.func("set_custom_source", ["name", "data"], "", |globals, args, _| {
            let mut args = args.into_iter();
            let name = args.next().unwrap().into_string()?;
            let data = args.next().unwrap().into_string()?;
            globals.set_custom_source(Source::new(name, None, data).into())?;
            Ok(Value::Nil)
        });
        m.func("remove_module", ["name"], "", |globals, args, _| {
            let mut args = args.into_iter();
            let name = args.next().unwrap().into_string()?;
            Ok(globals.remove_module(&name).map(Value::from).unwrap_or(Value::Nil))
        });
    })
}
