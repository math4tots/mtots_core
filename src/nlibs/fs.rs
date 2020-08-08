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
use std::path::MAIN_SEPARATOR;

pub(super) fn new() -> NativeModule {
    NativeModule::new("a.fs", |builder| {
        let builder = builder
            .doc("Module for interacting with the file system")
            .group("path", |builder| builder
                .val(
                    "sep",
                    concat!(
                        "File path separator\n",
                        "Basically \\ on windows and / everywhere else",
                    ),
                    MAIN_SEPARATOR,
                )
                .func(
                    "dirname",
                    ["path"],
                    concat!(
                        "Gets the directory component of a path ",
                        "(essentially a path's parent)",
                    ),
                    |_globals, args, _| {
                        let arg = args.into_iter().next().unwrap();
                        let path = Path::new(arg.string()?);
                        match path.parent() {
                            Some(parent) => Value::try_from(parent.as_os_str()),
                            None => Ok(Value::from("")),
                        }
                    },
                )
                .func(
                    "basename",
                    ["path"],
                    concat!("Gets the file name of a path",),
                    |_globals, args, _| {
                        let arg = args.into_iter().next().unwrap();
                        let path = Path::new(arg.string()?);
                        match path.file_name() {
                            Some(name) => Value::try_from(name),
                            None => Ok(Value::from("")),
                        }
                    },
                )
                .func(
                    "stem",
                    ["path"],
                    concat!("Extracts the stem (non-extension) portion of a paths' file name",),
                    |_globals, args, _| {
                        let arg = args.into_iter().next().unwrap();
                        let path = Path::new(arg.string()?);
                        match path.file_stem() {
                            Some(name) => Value::try_from(name),
                            None => Ok(Value::from("")),
                        }
                    },
                )
                .func(
                    "relpath",
                    ["start", "end"],
                    concat!("Returns the relative path that when joined with start will result in end",),
                    |_globals, args, _| {
                        let mut args = args.into_iter();
                        let startval = args.next().unwrap();
                        let start = Path::new(startval.string()?);
                        let endval = args.next().unwrap();
                        let end = Path::new(endval.string()?);
                        if let Some(common) = common_path(end, start) {
                            let mut ret = PathBuf::new();
                            for _ in 0..start.strip_prefix(common).unwrap().iter().count() {
                                ret.push("..");
                            }
                            ret.push(end.strip_prefix(common).unwrap());
                            Ok(Value::try_from(ret.into_os_string())?)
                        } else {
                            // If there's no common ancestor, there's no way to get
                            // from one to the other, so just return itself
                            Ok(endval)
                        }
                    },
                )
                .func(
                    "join",
                    ArgSpec::builder().var("parts"),
                    concat!("Creates a new path from parts",),
                    |_globals, args, _| {
                        let mut path = PathBuf::new();
                        for part in args {
                            let part = part.string()?;
                            path.push(part.str());
                        }
                        Value::try_from(path.into_os_string())
                    },
                )
            );

        let builder = builder.group("io", |builder| {
            builder
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
        });

        // functions that interact with the file system
        builder
            .group("files", |builder| {
                builder
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
                        "canon",
                        ["path"],
                        concat!(
                            "Returns the canonical, absolute form of a path with all ",
                            "intermediate components normalized and symbolic links ",
                            "resolved\n",
                            "Will throw if the resulting path is not valid UTF-8",
                        ),
                        |_globals, args, _| {
                            let arg = args.into_iter().next().unwrap();
                            let path = Path::new(arg.string()?);
                            let path = path.canonicalize()?;
                            Ok(Value::try_from(path.into_os_string())?)
                        },
                    )
                    .func(
                        "cwd",
                        (),
                        "Returns the current working directory",
                        |_globals, _args, _| {
                            Ok(Value::try_from(env::current_dir()?.into_os_string())?)
                        },
                    )
                    .func(
                        "ls",
                        ArgSpec::builder()
                            .req("dir")
                            .def("path", false)
                            .def("sort", false),
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
                            let sort = args.next().unwrap().truthy();
                            let mut paths = Vec::<Value>::new();
                            for entry in dir.read_dir()? {
                                let entry = entry?;
                                paths.push(if path {
                                    Value::try_from(entry.path().into_os_string())?
                                } else {
                                    Value::try_from(entry.file_name())?
                                });
                            }
                            if sort {
                                paths.sort_by(|a, b| a.partial_cmp(b).unwrap());
                            }
                            Ok(paths.into())
                        },
                    )
                    .func(
                        "walk",
                        ArgSpec::builder().req("top").def("sort", false),
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
                            let mut args = args.into_iter();
                            let top = PathBuf::from(args.next().unwrap().unwrap_or_clone_string()?);
                            let mut stack = vec![top];
                            let sort = args.next().unwrap().truthy();
                            Ok(Value::from(NativeGenerator::new(
                                "fs.walk",
                                move |_globals, _| {
                                    if let Some(dirpath) = stack.pop() {
                                        let mut filenames = Vec::new();
                                        let mut dirnames = Vec::new();

                                        macro_rules! process_entry {
                                            ($entry:expr) => {
                                                let entry = $entry;
                                                let name =
                                                    gentry!(Value::try_from(entry.file_name()));
                                                if gentry!(entry.file_type()).is_dir() {
                                                    dirnames.push(name);
                                                    stack.push(entry.path());
                                                } else {
                                                    filenames.push(name);
                                                }
                                            };
                                        }

                                        if sort {
                                            let mut entries =
                                                gentry!(gentry!(fs::read_dir(&dirpath))
                                                    .collect::<std::result::Result<Vec<_>, _>>(
                                                ));
                                            // sort in reverse order, so that when we pop from the stack
                                            // we get them in alphabetical order
                                            entries
                                                .sort_by(|a, b| b.file_name().cmp(&a.file_name()));
                                            for entry in entries {
                                                process_entry!(entry);
                                            }
                                        } else {
                                            for entry in gentry!(fs::read_dir(&dirpath)) {
                                                let entry = gentry!(entry);
                                                process_entry!(entry);
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
                        ArgSpec::builder().req("top").def("sort", false),
                        concat!(
                            "Walk the entire tree, yielding a path for every non-directory ",
                            "file along the way\n\n",
                            "NOTE: empty directories are effectively ignored and directories ",
                            "themselves are never visited on, only files.\n",
                            "If you need to visit directories, you might want the fs.walk() ",
                            "function instead\n",
                        ),
                        |_globals, args, _| {
                            let mut args = args.into_iter();
                            let top = PathBuf::from(args.next().unwrap().unwrap_or_clone_string()?);
                            let sort = args.next().unwrap().truthy();
                            let mut dirs = vec![top];
                            let mut files = vec![];
                            Ok(Value::from(NativeGenerator::new(
                                "fs.files",
                                move |_globals, _| {
                                    while files.is_empty() {
                                        if let Some(dir) = dirs.pop() {
                                            macro_rules! process_entry {
                                                ($entry:expr) => {
                                                    let entry = $entry;
                                                    let path = entry.path();
                                                    if gentry!(entry.file_type()).is_dir() {
                                                        dirs.push(path);
                                                    } else {
                                                        files.push(gentry!(Value::try_from(
                                                            path.into_os_string()
                                                        )));
                                                    }
                                                };
                                            }

                                            if sort {
                                                let mut entries =
                                                    gentry!(gentry!(fs::read_dir(&dir))
                                                        .collect::<std::result::Result<Vec<_>, _>>(
                                                    ));
                                                // sort in reverse order, so that when we pop from the stack
                                                // we get them in alphabetical order
                                                entries.sort_by(|a, b| {
                                                    b.file_name().cmp(&a.file_name())
                                                });
                                                for entry in entries {
                                                    process_entry!(entry);
                                                }
                                            } else {
                                                for entry in gentry!(fs::read_dir(&dir)) {
                                                    let entry = gentry!(entry);
                                                    process_entry!(entry);
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
            })
            .build()
    })
}

fn common_path<'a>(a: &'a Path, b: &Path) -> Option<&'a Path> {
    let mut cur = Some(a);
    while let Some(new_path) = cur {
        if b.starts_with(new_path) {
            return Some(new_path);
        }
        cur = new_path.parent();
    }
    None
}
