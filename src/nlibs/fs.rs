use crate::ArgSpec;
use crate::NativeModule;
use crate::Value;
use std::path::Path;
use std::convert::TryFrom;

pub(super) fn new() -> NativeModule {
    NativeModule::builder("a.fs")
        .func("isfile", ["path"], None, |_globals, args, _| {
            let arg = args.into_iter().next().unwrap();
            let path = Path::new(arg.string()?);
            Ok(Value::from(path.is_file()))
        })
        .func("isdir", ["path"], None, |_globals, args, _| {
            let arg = args.into_iter().next().unwrap();
            let path = Path::new(arg.string()?);
            Ok(Value::from(path.is_dir()))
        })
        .func("ls", ArgSpec::builder().req("dir").def("path", false), None, |_globals, args, _| {
            let mut args = args.into_iter();
            let pathval = args.next().unwrap();
            let dir = Path::new(pathval.string()?);
            let path = args.next().unwrap().truthy();
            let mut paths = Vec::<Value>::new();
            for entry in dir.read_dir()? {
                let entry = entry?;
                paths.push(if path {
                    Value::try_from(entry.path().into_os_string())?
                } else {
                    Value::try_from(entry.file_name())?
                });
            }
            Ok(paths.into())
        })
        .build()
}
