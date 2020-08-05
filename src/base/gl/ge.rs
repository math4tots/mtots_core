use super::*;

pub enum GlobalElement {
    NativeModule(NativeModule),
    SourceRoot(RcStr),
}

impl Globals {
    pub fn add<E: Into<GlobalElement>>(&mut self, e: E) -> Result<()> {
        match e.into() {
            GlobalElement::NativeModule(nm) => {
                self.native_modules.insert(nm.name().clone(), nm);
            }
            GlobalElement::SourceRoot(root) => {
                self.source_roots.push(root);
            }
        }
        Ok(())
    }
}

impl From<NativeModule> for GlobalElement {
    fn from(nm: NativeModule) -> Self {
        Self::NativeModule(nm)
    }
}

impl From<NativeModuleBuilder> for GlobalElement {
    fn from(nm: NativeModuleBuilder) -> Self {
        Self::NativeModule(nm.build())
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
