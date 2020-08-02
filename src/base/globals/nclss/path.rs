use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::NativeFunction;
use crate::Symbol;
use crate::Value;

use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;

pub(super) fn mkcls(base: Rc<Class>) -> Rc<Class> {
    let methods = vec![
        // ## Methods that do NOT touch the file system (pure path manip methods) ##
        NativeFunction::simple0("parent", &["self"], |globals, args, _kwargs| {
            let path = Eval::expect_path(globals, &args[0])?;
            Ok(path.parent().map(|p| p.into()).unwrap_or(Value::Nil))
        }),
        NativeFunction::simple0("basename", &["self"], |globals, args, _kwargs| {
            let path = Eval::expect_path(globals, &args[0])?;
            let basename = match path.file_name() {
                Some(basename) => Value::String(Eval::osstr_to_str(globals, basename)?.into()),
                None => Value::Nil,
            };
            Ok(basename)
        }),
        NativeFunction::snew(
            "join",
            (&["self"], &[], Some("args"), None),
            |globals, args, _kwargs| {
                let mut args = args.into_iter();
                let mut path = Eval::move_or_clone_path(globals, args.next().unwrap())?;
                for arg in args {
                    let part = Eval::expect_pathlike(globals, &arg)?;
                    path.push(part)
                }
                Ok(path.into())
            },
        ),
        NativeFunction::simple0("relpath", &["self", "start"], |globals, args, _kwargs| {
            // returns the relative path version of self, when starting from 'start'
            // NOTE: this is purely a string manipulation and doesn't take into
            // account, e.g. symlinks
            // returns self unchanged if the two paths share no common ancestor
            let path = Eval::expect_path(globals, &args[0])?;
            let start = Eval::expect_pathlike(globals, &args[1])?;
            if let Some(common) = common_path(path, start) {
                let mut ret = PathBuf::new();
                for _ in 0..start.strip_prefix(common).unwrap().iter().count() {
                    ret.push("..");
                }
                ret.push(path.strip_prefix(common).unwrap());
                Ok(ret.into())
            } else {
                // If there's no common ancestor, there's no way to get
                // from one to the other, so just return itself
                Ok(args[0].clone())
            }
        }),
        // ## Methods that touch the file system ##
        NativeFunction::simple0("is_file", &["self"], |globals, args, _kwargs| {
            let path = Eval::expect_path(globals, &args[0])?;
            Ok(path.is_file().into())
        }),
        NativeFunction::simple0("is_dir", &["self"], |globals, args, _kwargs| {
            let path = Eval::expect_path(globals, &args[0])?;
            Ok(path.is_dir().into())
        }),
        NativeFunction::simple0("canon", &["self"], |globals, args, _kwargs| {
            // Return the canonicalized version of this path
            // Resolves symlinks -- so this touches the filesystem and may
            // throw an IOError
            let path = Eval::expect_path(globals, &args[0])?;
            let canon = Eval::try_(globals, path.canonicalize())?;
            Ok(canon.into())
        }),
        NativeFunction::simple0("list", &["self"], |globals, args, _kwargs| {
            // lists a directory
            let mut children = Vec::new();
            let path = Eval::expect_path(globals, &args[0])?;
            for entry in Eval::try_(globals, path.read_dir())? {
                let entry = Eval::try_(globals, entry)?;
                children.push(Value::Path(entry.path().into()));
            }
            Ok(Value::List(children.into()))
        }),
        NativeFunction::simple0("rename", &["from", "to"], |globals, args, _kwargs| {
            // renames a file (e.g. mv)
            let from = Eval::expect_path(globals, &args[0])?;
            let to = Eval::expect_pathlike(globals, &args[1])?;
            Eval::try_(globals, fs::rename(from, to))?;
            Ok(Value::Nil)
        }),
        NativeFunction::simple0("read", &["self"], |globals, args, _kwargs| {
            // read the entire contents of a file to a string
            let path = Eval::expect_path(globals, &args[0])?;
            let string = Eval::try_(globals, fs::read_to_string(path))?;
            Ok(string.into())
        }),
        NativeFunction::simple0("read_bytes", &["self"], |globals, args, _kwargs| {
            // read the entire contents of a file to a string
            let path = Eval::expect_path(globals, &args[0])?;
            let bytes = Eval::try_(globals, fs::read(path))?;
            Ok(Value::Bytes(bytes.into()))
        }),
        NativeFunction::simple0("write", &["self", "data"], |globals, args, _kwargs| {
            // create a file or replace the contents of an existing file
            let path = Eval::expect_path(globals, &args[0])?;
            match &args[1] {
                Value::String(s) => Eval::try_(globals, fs::write(path, s.str()))?,
                Value::Bytes(b) => Eval::try_(globals, fs::write(path, &**b))?,
                _ => {
                    Eval::expect_bytes(globals, &args[1])?;
                    panic!("Path.write should have panic'd")
                }
            }
            Ok(Value::Nil)
        }),
        NativeFunction::simple0("remove_file", &["self"], |globals, args, _kwargs| {
            // removes a file
            let path = Eval::expect_path(globals, &args[0])?;
            Eval::try_(globals, fs::remove_file(path))?;
            Ok(Value::Nil)
        }),
        NativeFunction::simple0("remove_dir_all", &["self"], |globals, args, _kwargs| {
            // removes a directory after removing all its contents
            let path = Eval::expect_path(globals, &args[0])?;
            Eval::try_(globals, fs::remove_dir_all(path))?;
            Ok(Value::Nil)
        }),
        NativeFunction::simple0("remove_dir", &["self"], |globals, args, _kwargs| {
            // removes a directory
            let path = Eval::expect_path(globals, &args[0])?;
            Eval::try_(globals, fs::remove_dir(path))?;
            Ok(Value::Nil)
        }),
        NativeFunction::simple0("remove", &["self"], |globals, args, _kwargs| {
            // removes a file or directory
            let path = Eval::expect_path(globals, &args[0])?;
            if path.is_dir() {
                Eval::try_(globals, fs::remove_dir_all(path))?;
            } else {
                Eval::try_(globals, fs::remove_file(path))?;
            }
            Ok(Value::Nil)
        }),
        NativeFunction::simple0("mkdir", &["self"], |globals, args, _kwargs| {
            // creates a directory
            let path = Eval::expect_path(globals, &args[0])?;
            Eval::try_(globals, fs::create_dir(path))?;
            Ok(Value::Nil)
        }),
        NativeFunction::simple0("mkdirp", &["self"], |globals, args, _kwargs| {
            // creates a directory and all its parents as needed
            let path = Eval::expect_path(globals, &args[0])?;
            Eval::try_(globals, fs::create_dir_all(path))?;
            Ok(Value::Nil)
        }),
        NativeFunction::simple0("copy", &["from", "to"], |globals, args, _kwargs| {
            // Copies the contents of one file to another.
            // This function will also copy the permission bits of the original file to
            // the destination file.
            // Note that if from and to both point to the same file, then the file will
            // likely get truncated by this operation.
            let from = Eval::expect_path(globals, &args[0])?;
            let to = Eval::expect_pathlike(globals, &args[1])?;
            Eval::try_(globals, fs::copy(from, to))?;
            Ok(Value::Nil)
        }),
    ]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
    .collect();

    let static_methods = vec![NativeFunction::simple0(
        "__call",
        &["x"],
        |globals, args, _kwargs| Ok(Eval::expect_pathlike_rc(globals, &args[0])?.into()),
    )]
    .into_iter()
    .map(|f| (Symbol::from(f.name()), Value::from(f)))
    .collect();

    Class::new0(
        ClassKind::NativeClass,
        "Path".into(),
        vec![base],
        None,
        methods,
        static_methods,
    )
    .into()
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
