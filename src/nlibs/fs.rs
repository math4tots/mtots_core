use crate::ArgSpec;
use crate::NativeGenerator;
use crate::NativeModule;
use crate::ResumeResult;
use crate::Value;
use std::convert::TryFrom;
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

pub(super) fn new() -> NativeModule {
    NativeModule::new("a.fs", |builder| {
        builder
            .doc("Module for interacting with the file system")
            .func("isfile", ["path"], "", |_globals, args, _| {
                let arg = args.into_iter().next().unwrap();
                let path = Path::new(arg.string()?);
                Ok(Value::from(path.is_file()))
            })
            .func("isdir", ["path"], "", |_globals, args, _| {
                let arg = args.into_iter().next().unwrap();
                let path = Path::new(arg.string()?);
                Ok(Value::from(path.is_dir()))
            })
            .func(
                "cwd",
                (),
                "Returns the current working directory",
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
                    "while path=true may return ['src/foo', 'src/bar']\n\n",
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
            .func(
                "write",
                ["path", "data"],
                concat!(
                    "Writes data out into a file.\n",
                    "The data may be either a string or bytes object.\n",
                    "This function will create a file if it does not exist, ",
                    "and will entirely replace its contents if it does.\n",
                ),
                |_globals, args, _| {
                    let mut args = args.into_iter();
                    let arg = args.next().unwrap();
                    let path = Path::new(arg.string()?);
                    let data = args.next().unwrap();
                    if data.is_handle::<Vec<u8>>() {
                        let data = data.unwrap_or_clone_handle::<Vec<u8>>()?;
                        fs::write(path, data)?;
                    } else {
                        let data = data.string()?;
                        fs::write(path, data)?;
                    }
                    Ok(Value::Nil)
                },
            )
            .func(
                "walk",
                ["top"],
                concat!(
                    "Walk the entire tree, yielding a [dirpath, dirnames, filenames] triple ",
                    "at every directory along the way\n\n",
                    "  'dirpath' is a string, path to the directory\n",
                    "  'dirnames' is a list of strings containing the names of directories ",
                    "in the current directory (note: these are not full paths)\n",
                    "  'filenames' is a list of strings containing the names of non-directory ",
                    "files in the current directory\n",
                ),
                |_globals, args, _| {
                    let pathval = args.into_iter().next().unwrap();
                    let mut stack = vec![PathBuf::from(pathval.unwrap_or_clone_string()?)];
                    Ok(Value::from(NativeGenerator::new(
                        "fs.walk",
                        move |_globals, _| {
                            if let Some(dirpath) = stack.pop() {
                                let mut filenames = Vec::new();
                                let mut dirnames = Vec::new();
                                for entry in gentry!(fs::read_dir(&dirpath)) {
                                    let entry = gentry!(entry);
                                    let name = gentry!(Value::try_from(entry.file_name()));
                                    if gentry!(entry.file_type()).is_dir() {
                                        dirnames.push(name);
                                        stack.push(entry.path());
                                    } else {
                                        filenames.push(name);
                                    }
                                }
                                ResumeResult::Yield(
                                    vec![
                                        gentry!(Value::try_from(dirpath.into_os_string())),
                                        Value::from(dirnames),
                                        Value::from(filenames),
                                    ]
                                    .into(),
                                )
                            } else {
                                ResumeResult::Return(Value::Nil)
                            }
                        },
                    )))
                },
            )
            .func(
                "files",
                ["top"],
                concat!(
                    "Walk the entire tree, yielding a path for every non-directory ",
                    "file along the way\n\n",
                    "NOTE: empty directories are effectively ignored and directories ",
                    "themselves are never visited on, only files.\n",
                    "If you need to visit directories, you might want the fs.walk() ",
                    "function instead",
                ),
                |_globals, args, _| {
                    let pathval = args.into_iter().next().unwrap();
                    let mut dirs = vec![PathBuf::from(pathval.unwrap_or_clone_string()?)];
                    let mut files = vec![];
                    Ok(Value::from(NativeGenerator::new(
                        "fs.files",
                        move |_globals, _| {
                            while files.is_empty() {
                                if let Some(dir) = dirs.pop() {
                                    for entry in gentry!(fs::read_dir(&dir)) {
                                        let entry = gentry!(entry);
                                        let path = entry.path();
                                        if gentry!(entry.file_type()).is_dir() {
                                            dirs.push(path);
                                        } else {
                                            files.push(gentry!(Value::try_from(
                                                path.into_os_string()
                                            )));
                                        }
                                    }
                                } else {
                                    return ResumeResult::Return(Value::Nil);
                                }
                            }
                            ResumeResult::Yield(Value::from(files.pop().unwrap()))
                        },
                    )))
                },
            )
            .build()
    })
}
