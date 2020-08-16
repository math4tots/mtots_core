use crate::annotate;
use crate::compile;
use crate::ArgSpec;
use crate::Behavior;
use crate::Class;
use crate::ConstVal;
use crate::DocStr;
use crate::Error;
use crate::Handle;
use crate::HandleBehaviorBuilder;
use crate::Key;
use crate::LexErrorKind;
use crate::Lexer;
use crate::List;
use crate::Map;
use crate::Mark;
use crate::Module;
use crate::ModuleDisplay;
use crate::NativeFunction;
use crate::NativeGenerator;
use crate::Parser;
use crate::RcStr;
use crate::Result;
use crate::ResumeResult;
use crate::Set;
use crate::Source;
use crate::Value;
use std::any::Any;
use std::any::TypeId;
use std::cell::Ref;
use std::cell::RefCell;
use std::cell::RefMut;
use std::cmp;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::collections::HashSet;
use std::convert::TryFrom;
use std::path::PathBuf;
use std::rc::Rc;
mod bltn;
mod clss;
mod ge;
mod hist;
mod hnd;
mod load;
mod nm;
mod parse;
mod stash;
mod trampoline;
pub use clss::*;
pub use ge::*;
pub use nm::*;
pub use stash::*;

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
    custom_sources: HashMap<RcStr, Rc<Source>>,

    // builtins
    class_manager: ClassManager,
    builtins: HashMap<RcStr, Value>,
    repl_scope: Option<HashMap<RcStr, Rc<RefCell<Value>>>>,

    // interfacing with native types (i.e. Handles)
    handle_class_map: HashMap<TypeId, Rc<Class>>,

    // For storing arbitrary global data for various applications
    // You can use an application local type to store application
    // local information
    stash: Stash,

    // For application that needs to hijack the main thread (e.g. GUIs),
    // the Globals object might need to be moved.
    // The trampoline allows an application to request this from its host
    // environment.
    trampoline: Option<Box<dyn FnOnce(Globals)>>,

    // command line arguments; need to be explicitly set to be nonempty
    argv: Option<Vec<RcStr>>,

    // Just a best effort support and line editing experience with
    // rustyline.
    #[cfg(feature = "line")]
    line: rustyline::Editor<()>,

    // print handlers
    print: Option<Box<dyn Fn(&str)>>,
    eprint: Option<Box<dyn Fn(&str)>>,
}

impl Globals {
    pub fn new() -> Self {
        let class_manager = ClassManager::new();
        let builtins = Self::bootstrap_new_builtins(&class_manager);
        #[cfg(feature = "line")]
        let line = Self::new_line_editor();
        let mut globals = Self {
            trace: vec![],
            lexer: Lexer::new(),
            parser: Parser::new(),
            module_map: HashMap::new(),
            native_modules: HashMap::new(),
            source_roots: vec![],
            main_module: None,
            custom_sources: HashMap::new(),
            class_manager,
            builtins,
            repl_scope: None,
            handle_class_map: HashMap::new(),
            stash: Default::default(),
            trampoline: None,
            argv: None,
            #[cfg(feature = "line")]
            line,
            print: None,
            eprint: None,
        };
        globals.add_builtin_native_libraries();
        globals
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
        let path = source.path().clone();
        let mut display = self.parse(source)?;
        annotate(&mut display)?;
        let code = compile(&display)?;
        let mut map = self.builtins.clone();
        map.insert("__name".into(), name.into());
        if let Some(path) = path {
            if let Some(pathstr) = path.to_str() {
                map.insert("__file".into(), pathstr.into());
            }
        }
        code.apply_for_module(self, &map)
    }
    pub fn exec_str(&mut self, name: &str, path: Option<&str>, data: &str) -> Result<Rc<Module>> {
        self.exec(
            Source::new(
                name.into(),
                path.map(PathBuf::from).map(Rc::from),
                data.into(),
            )
            .into(),
        )
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
    pub fn argv(&self) -> &Option<Vec<RcStr>> {
        &self.argv
    }
    pub fn set_argv(&mut self, argv: Vec<RcStr>) {
        self.argv = Some(argv);
    }
    pub fn print(&self, text: &str) {
        match &self.print {
            Some(print) => print(text),
            None => print!("{}", text),
        }
    }
    pub fn eprint(&self, text: &str) {
        match &self.eprint {
            Some(eprint) => eprint(text),
            None => eprint!("{}", text),
        }
    }
    pub fn set_print<F>(&mut self, f: F)
    where
        F: Fn(&str) + 'static,
    {
        self.print = Some(Box::new(f));
    }
    pub fn set_eprint<F>(&mut self, f: F)
    where
        F: Fn(&str) + 'static,
    {
        self.eprint = Some(Box::new(f));
    }
}
