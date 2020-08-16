use super::*;

pub enum GlobalElement {
    NativeModule(NativeModule),
    SourceRoot(RcStr),
    Source(Rc<Source>),
}

impl Globals {
    pub fn add_native_module(&mut self, nm: NativeModule) -> Result<()> {
        if self.native_modules.contains_key(nm.name()) {
            panic!("Duplicate native module for {:?}", nm.name());
        }
        self.native_modules.insert(nm.name().clone(), nm);
        Ok(())
    }

    pub fn add_source_root<S: Into<RcStr>>(&mut self, root: S) {
        self.source_roots.push(root.into());
    }

    pub fn add_custom_source(&mut self, source: Rc<Source>) -> Result<()> {
        if self.custom_sources.contains_key(source.name()) {
            panic!("Duplicate custom source for {:?}", source.name());
        }
        self.set_custom_source(source)
    }

    pub fn set_custom_source(&mut self, source: Rc<Source>) -> Result<()> {
        self.custom_sources.insert(source.name().clone(), source);
        Ok(())
    }
}

impl From<NativeModule> for GlobalElement {
    fn from(nm: NativeModule) -> Self {
        Self::NativeModule(nm)
    }
}

impl From<RcStr> for GlobalElement {
    fn from(sr: RcStr) -> Self {
        Self::SourceRoot(sr)
    }
}

impl From<&RcStr> for GlobalElement {
    fn from(sr: &RcStr) -> Self {
        Self::SourceRoot(sr.clone())
    }
}

impl From<&str> for GlobalElement {
    fn from(sr: &str) -> Self {
        Self::SourceRoot(sr.into())
    }
}

impl From<&String> for GlobalElement {
    fn from(sr: &String) -> Self {
        Self::SourceRoot(sr.into())
    }
}

impl From<String> for GlobalElement {
    fn from(sr: String) -> Self {
        Self::SourceRoot(sr.into())
    }
}

impl From<Source> for GlobalElement {
    fn from(s: Source) -> Self {
        Self::Source(s.into())
    }
}

impl From<Rc<Source>> for GlobalElement {
    fn from(s: Rc<Source>) -> Self {
        Self::Source(s)
    }
}

impl From<&Rc<Source>> for GlobalElement {
    fn from(s: &Rc<Source>) -> Self {
        Self::Source(s.clone())
    }
}
