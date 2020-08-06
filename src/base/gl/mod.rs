use crate::annotate;
use crate::compile;
use crate::Class;
use crate::Error;
use crate::Handle;
use crate::LexErrorKind;
use crate::Lexer;
use crate::Mark;
use crate::Module;
use crate::ModuleDisplay;
use crate::NativeFunction;
use crate::Parser;
use crate::RcStr;
use crate::Result;
use crate::Source;
use crate::Value;
use std::any::Any;
use std::any::TypeId;
use std::cell::RefCell;
use std::cmp;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::rc::Rc;
mod bltn;
mod clss;
mod ge;
mod hnd;
mod load;
mod nm;
mod parse;
pub use clss::*;
pub use ge::*;
pub use nm::*;

/// The global state for mtots
pub struct Globals {
    // debug info (stack trace)
    trace: Vec<Mark>,

    // frontend stuff
    lexer: Lexer,
    parser: Parser,

    // module management
    module_map: HashMap<RcStr, Rc<Module>>,
    native_modules: HashMap<RcStr, NativeModule>,
    source_roots: Vec<RcStr>,
    main_module: Option<RcStr>,

    // builtins
    class_manager: ClassManager,
    builtins: HashMap<RcStr, Value>,
    repl_scope: Option<HashMap<RcStr, Rc<RefCell<Value>>>>,

    // interfacing with native types (i.e. Handles)
    handle_class_map: HashMap<TypeId, Rc<Class>>,
}

impl Globals {
    pub fn new() -> Self {
        let class_manager = ClassManager::new();
        let builtins = Self::new_builtins(&class_manager);
        Self {
            trace: vec![],
            lexer: Lexer::new(),
            parser: Parser::new(),
            module_map: HashMap::new(),
            native_modules: HashMap::new(),
            source_roots: vec![],
            main_module: None,
            class_manager,
            builtins,
            repl_scope: None,
            handle_class_map: HashMap::new(),
        }
    }
    pub fn trace(&self) -> &Vec<Mark> {
        &self.trace
    }
    pub fn trace_unwind(&mut self, len: usize) {
        self.trace.truncate(len);
    }
    pub(crate) fn trace_push(&mut self, mark: Mark) {
        self.trace.push(mark);
    }
    pub(crate) fn trace_pop(&mut self) {
        self.trace.pop().unwrap();
    }
    pub fn register_module(&mut self, module: Rc<Module>) -> Result<()> {
        let name = module.name().clone();
        if self.module_map.contains_key(&name) {
            return Err(rterr!("Module {:?} registered twice", name));
        }
        self.module_map.insert(name, module);
        Ok(())
    }
    pub fn class_manager(&self) -> &ClassManager {
        &self.class_manager
    }
    pub fn get_main(&self) -> &Option<RcStr> {
        &self.main_module
    }
    pub fn set_main(&mut self, main_module_name: RcStr) {
        self.main_module = Some(main_module_name);
    }
    pub fn exec(&mut self, source: Rc<Source>) -> Result<Rc<Module>> {
        let name = source.name().clone();
        let pathstr = source.path().clone().map(RcStr::from);
        let mut display = self.parse(source)?;
        annotate(&mut display)?;
        let code = compile(&display)?;
        let mut map = self.builtins.clone();
        map.insert("__name".into(), name.into());
        if let Some(pathstr) = pathstr {
            map.insert("__file".into(), pathstr.into());
        }
        code.apply_for_module(self, &map)
    }
    pub fn exec_str(&mut self, name: &str, path: Option<&str>, data: &str) -> Result<Rc<Module>> {
        self.exec(Source::new(name.into(), path.map(RcStr::from), data.into()).into())
    }
    pub fn exec_repl(&mut self, data: &str) -> Result<Value> {
        let mut display = self.parse(Rc::new(Source::new("[repl]".into(), None, data.into())))?;
        annotate(&mut display)?;
        let code = compile(&display)?;
        code.apply_for_repl(self)
    }
    pub(super) fn repl_scope_mut(&mut self) -> &mut HashMap<RcStr, Rc<RefCell<Value>>> {
        if self.repl_scope.is_none() {
            let mut scope = HashMap::new();
            for (key, val) in &self.builtins {
                scope.insert(key.clone(), Rc::new(RefCell::new(val.clone())));
            }
            self.repl_scope = Some(scope);
        }
        self.repl_scope.as_mut().unwrap()
    }
}