use crate::ArgSpec;
use crate::NativeModule;
use crate::Value;
use std::convert::TryFrom;
use std::env;
use std::fs;
use std::path::Path;

pub(super) fn new() -> NativeModule {
    NativeModule::new("a.fs", |_globals, builder| {
        builder
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
            .func(
                "cwd",
                (),
                "Reurns the current working directory",
                |_globals, _args, _| Ok(Value::try_from(env::current_dir()?.into_os_string())?),
            )
            .func(
                "ls",
                ArgSpec::builder().req("dir").def("path", false),
                concat!(
                    "Lists the files in a directory\n",
                    "By default this function will just return the filenames, ",
                    "but if the 'path' argument is set to true, each file will be ",
                    "prefixed by the given directory path as to form a path to the ",
                    "members of the directory\n\n",
                    "That is, if path=false, ls('src') may return ['foo', 'bar'], ",
                    "if path=true may return ['src/foo', 'src/bar']\n\n",
                    "Due to the requirement that mtots strings be valid UTF-8, ",
                    "this function will throw if any of the base names are not valid UTF-8\n",
                ),
                |_globals, args, _| {
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
                },
            )
            .func(
                "read",
                ["path"],
                concat!(
                    "Read the entire contents of a file into a string\n",
                    "Will throw an exception if the data is not valid UTF-8",
                ),
                |_globals, args, _| {
                    let mut args = args.into_iter();
                    let arg = args.next().unwrap();
                    let path = Path::new(arg.string()?);
                    let data = fs::read_to_string(path)?;
                    Ok(data.into())
                },
            )
            .build()
    })
}
