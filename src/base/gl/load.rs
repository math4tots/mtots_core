use super::*;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

impl Globals {
    pub fn load(&mut self, name: &RcStr) -> Result<&Rc<Module>> {
        if !self.module_map.contains_key(name) {
            self.load_uncached(name)?;
        }
        Ok(self.module_map.get(name).unwrap())
    }
    fn load_uncached(&mut self, name: &RcStr) -> Result<Rc<Module>> {
        if let Some(native_module) = self.native_modules.get_mut(name) {
            let init = match native_module.init() {
                Some(init) => init,
                None => return Err(rterr!("Exhausted native module {:?}", native_module.name())),
            };
            let module = Rc::new(Module::new(name.clone(), native_module.fields().clone()));
            self.register_module(module.clone())?;
            init(self, module.map())?;
            return Ok(module);
        }
        if let Some(source) = self.find_source(name)? {
            self.exec(source)?;
        }
        Err(rterr!("Module {:?} not found", name))
    }
    fn find_source(&self, name: &RcStr) -> Result<Option<Rc<Source>>> {
        if let Some(path) = self.find_source_path(name) {
            let srcpath = Some(format!("{:?}", path).into());
            let data = fs::read_to_string(path)?;
            Ok(Some(Source::new(name.clone(), srcpath, data.into()).into()))
        } else {
            Ok(None)
        }
    }
    fn find_source_path(&self, name: &RcStr) -> Option<PathBuf> {
        let relpaths = vec![
            {
                let mut path = PathBuf::new();
                for part in name.split('.') {
                    path.push(part);
                }
                path.push("__init.u");
                path
            },
            {
                let mut path = PathBuf::new();
                let mut iter = name.split('.').peekable();
                while let Some(part) = iter.next() {
                    if iter.peek().is_some() {
                        path.push(part);
                    } else {
                        path.push(format!("{}.u", part));
                    }
                }
                path
            },
        ];
        for root in &self.source_roots {
            for relpath in &relpaths {
                let path = Path::new(root.str()).join(relpath);
                if path.is_file() {
                    return Some(path);
                }
            }
        }
        None
    }
}