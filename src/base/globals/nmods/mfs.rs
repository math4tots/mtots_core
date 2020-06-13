//! JSON bindings
use crate::Eval;
use crate::EvalResult;
use crate::Globals;
use crate::HMap;
use crate::NativeFunction;
use crate::RcStr;
use crate::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;

pub const NAME: &str = "a._fs";

pub(super) fn load(globals: &mut Globals) -> EvalResult<HMap<RcStr, Rc<RefCell<Value>>>> {
    let sr = globals.symbol_registry();
    let mut map = HashMap::<RcStr, Value>::new();

    map.extend(
        vec![
            NativeFunction::simple0(sr, "copy_file", &["source", "dest"], |globals, args, _| {
                let source = Eval::expect_pathlike(globals, &args[0])?;
                let dest = Eval::expect_pathlike(globals, &args[1])?;
                Eval::try_(globals, fs::copy(source, dest))?;
                Ok(Value::Nil)
            }),
            NativeFunction::simple0(sr, "copy", &["source", "dest"], |globals, args, _| {
                let source = Eval::expect_pathlike(globals, &args[0])?;
                let dest = Eval::expect_pathlike(globals, &args[1])?;
                copy_tree(globals, source, dest)?;
                Ok(Value::Nil)
            }),
        ]
        .into_iter()
        .map(|f| (f.name().clone(), f.into())),
    );

    Ok({
        let mut ret = HMap::new();
        for (key, value) in map {
            ret.insert(key, Rc::new(RefCell::new(value)));
        }
        ret
    })
}

// Tries to copy a set of files from source to dest
// It will fail if it encounters any symlinks
fn copy_tree(globals: &mut Globals, source: &Path, dest: &Path) -> EvalResult<()> {
    let mut stack: Vec<(PathBuf, PathBuf)> = vec![(source.into(), dest.into())];
    while let Some((source, dest)) = stack.pop() {
        let metadata = Eval::try_(globals, source.symlink_metadata())?;
        let filetype = metadata.file_type();
        if filetype.is_symlink() {
            // TODO: Implement copy_symlink
            return globals.set_exc_str(
                "Symlink encountered when trying to copy (use copy_file or copy_symlink instead)",
            );
        } else if filetype.is_dir() {
            Eval::try_(globals, fs::create_dir_all(&dest))?;
            for entry in Eval::try_(globals, fs::read_dir(&source))? {
                let entry = Eval::try_(globals, entry)?;
                let path = entry.path();
                let relpath = path.strip_prefix(&source).unwrap();
                let sub_dest = dest.join(relpath);
                stack.push((path, sub_dest));
            }
        } else if filetype.is_file() {
            Eval::try_(globals, fs::copy(source, dest))?;
        } else {
            return globals.set_os_error(
                &format!(
                    "File type of {:?} ({:?}) could not be identified",
                    dest, filetype,
                )
                .into(),
            );
        }
    }
    Ok(())
}
