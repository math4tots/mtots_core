use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::NativeFunction;
use crate::SymbolRegistryHandle;
use crate::Value;

use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;

pub(super) fn mkcls(sr: &SymbolRegistryHandle, base: Rc<Class>) -> Rc<Class> {
    let methods = vec![
        // ## Methods that do NOT touch the file system (pure path manip methods) ##
        NativeFunction::simple0(sr, "parent", &["self"], |globals, args, _kwargs| {
            let path = Eval::expect_path(globals, &args[0])?;
            Ok(path.parent().map(|p| p.into()).unwrap_or(Value::Nil))
        }),
        NativeFunction::simple0(sr, "basename", &["self"], |globals, args, _kwargs| {
            let path = Eval::expect_path(globals, &args[0])?;
            let basename = match path.file_name() {
                Some(basename) => Value::String(Eval::osstr_to_str(globals, basename)?.into()),
                None => Value::Nil,
            };
            Ok(basename)
        }),
        NativeFunction::simple0(
            sr,
            "join",
            &["self", "relpath"],
            |globals, args, _kwargs| {
                let path = Eval::expect_path(globals, &args[0])?;
                let relpath = Eval::expect_pathlike(globals, &args[1])?;
                Ok(path.join(relpath).into())
            },
        ),
        NativeFunction::simple0(
            sr,
            "relpath",
            &["self", "start"],
            |globals, args, _kwargs| {
                // returns the relative path version of self, when starting from 'start'
                // NOTE: this is purely a string manipulation and doesn't take into
                // account, e.g. symlinks
                // returns self unchanged if the two paths share no common ancestor
                let path = Eval::expect_path(globals, &args[0])?;
                let start = Eval::expect_pathlike(globals, &args[1])?;
                if let Some(common) = common_path(path, &start) {
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
            },
        ),
        // ## Methods that touch the file system ##
        NativeFunction::simple0(sr, "is_file", &["self"], |globals, args, _kwargs| {
            let path = Eval::expect_path(globals, &args[0])?;
            Ok(path.is_file().into())
        }),
        NativeFunction::simple0(sr, "is_dir", &["self"], |globals, args, _kwargs| {
            let path = Eval::expect_path(globals, &args[0])?;
            Ok(path.is_dir().into())
        }),
        NativeFunction::simple0(sr, "canon", &["self"], |globals, args, _kwargs| {
            // Return the canonicalized version of this path
            // Resolves symlinks -- so this touches the filesystem and may
            // throw an IOError
            let path = Eval::expect_path(globals, &args[0])?;
            let canon = Eval::try_(globals, path.canonicalize())?;
            Ok(canon.into())
        }),
        NativeFunction::simple0(sr, "list", &["self"], |globals, args, _kwargs| {
            // lists a directory
            let mut children = Vec::new();
            let path = Eval::expect_path(globals, &args[0])?;
            for entry in Eval::try_(globals, path.read_dir())? {
                let entry = Eval::try_(globals, entry)?;
                children.push(Value::Path(entry.path().into()));
            }
            Ok(Value::List(children.into()))
        }),
    ]
    .into_iter()
    .map(|f| (sr.intern_rcstr(f.name()), Value::from(f)))
    .collect();

    let static_methods = vec![NativeFunction::simple0(
        sr,
        "new",
        &["x"],
        |globals, args, _kwargs| Ok(Eval::expect_pathlike(globals, &args[0])?.into()),
    )]
    .into_iter()
    .map(|f| (sr.intern_rcstr(f.name()), Value::from(f)))
    .collect();

    Class::new0(
        ClassKind::NativeClass,
        "Path".into(),
        vec![base],
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