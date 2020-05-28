use crate::EvalResult;
use crate::Globals;
use crate::HMap;
use crate::RcPath;
use crate::RcStr;
use crate::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;
use std::rc::Rc;

pub const SOURCE_FILE_EXTENSION: &str = "u";
pub const MTOTS_PATH: &str = "MTOTS_PATH";

#[derive(Debug)]
pub enum SourceFinderError {
    SourceNotFound,
    ConflictingModulePaths(Vec<RcPath>),
    IOError(io::Error),
}

pub struct SourceFinder {
    roots: Vec<RcPath>,
    cache: HashMap<RcStr, SourceItem>,
}

fn os_sep() -> &'static str {
    if cfg!(windows) {
        ";"
    } else {
        ":"
    }
}

impl SourceFinder {
    pub fn new() -> SourceFinder {
        SourceFinder {
            roots: Vec::new(),
            cache: HashMap::new(),
        }
    }

    pub fn add_root(&mut self, root: RcPath) {
        self.roots.push(root);
    }

    pub fn add_roots_from_str(&mut self, roots_str: &str) {
        for root in roots_str.split(os_sep()) {
            self.add_root(Path::new(root).into());
        }
    }

    pub fn add_roots_from_env(&mut self) -> Result<(), std::env::VarError> {
        // Ideally, we would use OsStr so that we can still add source roots even
        // when the paths are not valid unicode. However, the problem is that right now
        // OsStr in Rust is fairly opaque and we can't even split such a string easily
        self.add_roots_from_str(&std::env::var(MTOTS_PATH)?);
        Ok(())
    }

    pub fn add_file(&mut self, name: RcStr, path: RcPath) -> Result<(), std::io::Error> {
        let data = std::fs::read_to_string(&path)?.into();
        self.cache.insert(name, SourceItem::File { path, data });
        Ok(())
    }

    pub fn add_embedded_source(&mut self, name: RcStr, data: &'static str) {
        self.cache.insert(name, SourceItem::Embedded { data });
    }

    pub fn add_native<F>(&mut self, name: RcStr, f: F)
    where
        F: FnOnce(&mut Globals) -> EvalResult<HMap<RcStr, Rc<RefCell<Value>>>> + 'static,
    {
        self.cache
            .insert(name, SourceItem::Native { body: Box::new(f) });
    }

    pub(crate) fn load(&mut self, name: &RcStr) -> Result<SourceItem, SourceFinderError> {
        if let Some(item) = self.cache.remove(name) {
            Ok(item)
        } else {
            self.load_nocache(name)
        }
    }

    fn load_nocache(&mut self, name: &RcStr) -> Result<SourceItem, SourceFinderError> {
        for root in &self.roots {
            let path = get_file_path_for_module(root, name)?;
            if path.is_file() {
                let data = match fs::read_to_string(&path) {
                    Ok(data) => data.into(),
                    Err(error) => return Err(SourceFinderError::IOError(error)),
                };
                return Ok(SourceItem::File { path, data });
            }
        }
        Err(SourceFinderError::SourceNotFound)
    }
}

fn get_file_path_for_module(root: &Path, name: &str) -> Result<RcPath, SourceFinderError> {
    // foo/bar/__init.u
    let folder_path = get_file_nested_path_for_module(root, name);

    // foo/bar.u
    let short_path = get_file_short_path_for_module(root, name);

    if folder_path.is_file() && short_path.is_file() {
        Err(SourceFinderError::ConflictingModulePaths(vec![
            folder_path,
            short_path,
        ]))
    } else if folder_path.is_file() {
        Ok(folder_path)
    } else {
        Ok(short_path)
    }
}

fn get_file_short_path_for_module(root: &Path, name: &str) -> RcPath {
    let mut path = root.to_owned();
    let mut parts = name.split(".").peekable();
    while let Some(part) = parts.next() {
        if parts.peek().is_some() {
            path.push(part);
        } else {
            path.push(format!("{}.{}", part, SOURCE_FILE_EXTENSION));
        }
    }
    path.into()
}

fn get_file_nested_path_for_module(root: &Path, name: &str) -> RcPath {
    let mut path = root.to_owned();
    let mut parts = name.split(".");
    while let Some(part) = parts.next() {
        path.push(part);
    }
    path.push("__init.u");
    path.into()
}

pub(crate) enum SourceItem {
    File {
        path: RcPath,
        data: RcStr,
    },
    Native {
        body: Box<dyn FnOnce(&mut Globals) -> EvalResult<HMap<RcStr, Rc<RefCell<Value>>>>>,
    },
    Embedded {
        data: &'static str,
    },
}
