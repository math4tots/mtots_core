use crate::compile;
use crate::Class;
use crate::ClassKind;
use crate::EvalError;
use crate::EvalResult;
use crate::Expression;
use crate::Frame;
use crate::FrameError;
use crate::HMap;
use crate::Handle;
use crate::LexError;
use crate::LexErrorKind;
use crate::Lexer;
use crate::Module;
use crate::NativeFunction;
use crate::Parser;
use crate::RcPath;
use crate::RcStr;
use crate::ReplDelegate;
use crate::ReplScope;
use crate::SourceName;
use crate::Symbol;
use crate::Token;
use crate::Value;
use crate::ValueKind;
use std::any::Any;
use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

mod bfuncs;
mod builtins;
mod exc;
mod finder;
mod nclss;
mod nmods;
mod stash;

use finder::SourceFinder;
use finder::SourceFinderError;
use finder::SourceItem;
pub use stash::Stashable;

pub use bfuncs::NativeFunctions;
pub use exc::BuiltinExceptions;
pub use exc::Exception;
pub use exc::ExceptionKind;
pub use exc::ExceptionRegistry;
pub use finder::SOURCE_FILE_EXTENSION;
pub use nclss::BuiltinClasses;

#[derive(Debug)]
pub struct ErrorIndicator;

#[allow(dead_code)]
pub struct Globals {
    trace: Vec<(RcStr, usize)>,                 // (module-name, lineno) pairs
    line_cache: HashMap<(RcStr, usize), RcStr>, // maps (module-name, lineno) to lines
    exc: Option<Exception>,

    builtins: HashMap<RcStr, Value>,
    builtin_classes: BuiltinClasses,
    builtin_functions: NativeFunctions,
    module_registry: HashMap<RcStr, Rc<Module>>,
    filepath_registry: HashMap<RcStr, RcPath>, // maps modules to their paths
    source_registry: HashMap<RcStr, RcStr>,    // maps modules to their sources
    finder: SourceFinder,
    lexer: Lexer,
    parser: Parser,
    cli_args: Vec<RcStr>,
    main_module_name: Option<RcStr>,

    /// NOTE: the callback doesn't return an EvalResult, because the Globals
    /// is consumed anyway, and so there's no nice way to dump the error anyway.
    /// In fact, ideally, I would return a '!' type here, it's currently the '!'
    /// type is considered unstable here.
    trampoline_callback: Option<Box<dyn FnOnce(Globals)>>,

    /// Used by 'new' to determine what class to instantiate.
    /// The usize value records the length of trace when the corresponding
    /// class is added to the stack, so that we can prevent functions
    /// called from inside __call from inadvertently gaining access
    /// to the class.
    new_stack: Vec<(Rc<Class>, usize)>,

    /// Stash for storing arbitrary global values.
    stash: HashMap<TypeId, Rc<dyn Any>>,

    /// Sets
    handle_class_map: HashMap<TypeId, Rc<Class>>,

    exception_registry: ExceptionRegistry,
    builtin_exceptions: BuiltinExceptions,

    symbol_dunder_str: Symbol,
    symbol_dunder_repr: Symbol,
    symbol_dunder_add: Symbol,
    symbol_dunder_sub: Symbol,
    symbol_dunder_mul: Symbol,
    symbol_dunder_div: Symbol,
    symbol_dunder_truncdiv: Symbol,
    symbol_dunder_rem: Symbol,
    symbol_dunder_eq: Symbol,
    symbol_dunder_lt: Symbol,
    symbol_little: Symbol,
    symbol_big: Symbol,
    char_cache: Vec<Value>,
}

impl Globals {
    pub fn new() -> Globals {
        let (exception_registry, builtin_exceptions) = exc::new();
        let builtin_classes = Self::new_builtin_classes();
        let builtin_functions = bfuncs::new();
        let symbol_dunder_str = Symbol::from("__str");
        let symbol_dunder_repr = Symbol::from("__repr");
        let symbol_dunder_add = Symbol::from("__add");
        let symbol_dunder_sub = Symbol::from("__sub");
        let symbol_dunder_mul = Symbol::from("__mul");
        let symbol_dunder_div = Symbol::from("__div");
        let symbol_dunder_truncdiv = Symbol::from("__truncdiv");
        let symbol_dunder_rem = Symbol::from("__rem");
        let symbol_dunder_eq = Symbol::from("__eq");
        let symbol_dunder_lt = Symbol::from("__lt");
        let symbol_little = Symbol::from("little");
        let symbol_big = Symbol::from("big");
        let char_cache = {
            let mut cache = Vec::<Value>::new();
            for i in 0..128 {
                let c = (i as u8) as char;
                cache.push(format!("{}", c).into());
            }
            cache
        };
        let mut globals = Globals {
            trace: Vec::new(),
            line_cache: HashMap::new(),
            exc: None,
            builtins: Self::new_builtins(&builtin_classes, &builtin_functions, &builtin_exceptions),
            builtin_classes,
            builtin_functions,
            module_registry: HashMap::new(),
            filepath_registry: HashMap::new(),
            source_registry: HashMap::new(),
            finder: SourceFinder::new(),
            lexer: Lexer::new(),
            parser: Parser::new(),
            cli_args: Vec::new(),
            main_module_name: None,
            trampoline_callback: None,
            new_stack: Vec::new(),
            stash: HashMap::new(),
            handle_class_map: HashMap::new(),
            exception_registry,
            builtin_exceptions,
            symbol_dunder_repr,
            symbol_dunder_str,
            symbol_dunder_add,
            symbol_dunder_sub,
            symbol_dunder_mul,
            symbol_dunder_div,
            symbol_dunder_truncdiv,
            symbol_dunder_rem,
            symbol_dunder_eq,
            symbol_dunder_lt,
            symbol_little,
            symbol_big,
            char_cache,
        };
        globals.add_builtin_native_modules();
        super::emb::install_embedded_sources(&mut globals);
        globals
    }

    pub fn char_to_val(&self, ch: char) -> Value {
        if ch < (128 as char) {
            self.char_cache[ch as usize].clone()
        } else {
            format!("{}", ch).into()
        }
    }

    pub fn symbol_dunder_str(&self) -> Symbol {
        self.symbol_dunder_str
    }

    pub fn symbol_dunder_repr(&self) -> Symbol {
        self.symbol_dunder_repr
    }

    pub fn symbol_dunder_add(&self) -> Symbol {
        self.symbol_dunder_add
    }

    pub fn symbol_dunder_sub(&self) -> Symbol {
        self.symbol_dunder_sub
    }

    pub fn symbol_dunder_mul(&self) -> Symbol {
        self.symbol_dunder_mul
    }

    pub fn symbol_dunder_div(&self) -> Symbol {
        self.symbol_dunder_div
    }

    pub fn symbol_dunder_truncdiv(&self) -> Symbol {
        self.symbol_dunder_truncdiv
    }

    pub fn symbol_dunder_rem(&self) -> Symbol {
        self.symbol_dunder_rem
    }

    pub fn symbol_dunder_eq(&self) -> Symbol {
        self.symbol_dunder_eq
    }

    pub fn symbol_dunder_lt(&self) -> Symbol {
        self.symbol_dunder_lt
    }

    pub fn symbol_little(&self) -> Symbol {
        self.symbol_little
    }

    pub fn symbol_big(&self) -> Symbol {
        self.symbol_big
    }

    pub(crate) fn trace_push(&mut self, module_name: RcStr, lineno: usize) {
        self.trace.push((module_name, lineno));
    }

    pub(crate) fn trace_pop(&mut self) {
        self.trace.pop();
    }

    pub fn trace_len(&self) -> usize {
        self.trace.len()
    }

    pub(crate) fn trace_trunc(&mut self, new_len: usize) {
        self.trace.truncate(new_len);
    }

    pub fn trace(&self) -> &Vec<(RcStr, usize)> {
        &self.trace
    }

    pub fn translated_trace(&mut self) -> Vec<(SourceName, usize, Option<RcStr>)> {
        let mut new_trace = Vec::new();
        for (module_name, lineno) in &self.trace {
            let module_name = module_name.clone();
            let lineno = *lineno;

            let source_name = match self.filepath_registry.get(&module_name) {
                Some(filepath) => SourceName::File(filepath.clone()),
                None => SourceName::ModuleName(module_name.clone()),
            };
            let line = Self::get_line(
                &mut self.line_cache,
                &self.source_registry,
                module_name.clone(),
                lineno,
            );
            new_trace.push((source_name, lineno, line));
        }
        new_trace
    }

    fn get_line(
        line_cache: &mut HashMap<(RcStr, usize), RcStr>,
        source_registry: &HashMap<RcStr, RcStr>,
        module_name: RcStr,
        lineno: usize,
    ) -> Option<RcStr> {
        let key = (module_name.clone(), lineno);
        if !line_cache.contains_key(&key) {
            let source = source_registry.get(&module_name)?;
            let line: RcStr = source.lines().nth(lineno - 1)?.into();
            line_cache.insert(key.clone(), line);
        }
        line_cache.get(&key).map(|line| line.clone())
    }

    pub fn trace_fmt<'a>(&'a mut self) -> impl std::fmt::Display + 'a {
        struct Disp<'a> {
            globals: RefCell<&'a mut Globals>,
        }
        impl<'a> std::fmt::Display for Disp<'a> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                for (source_name, lineno, line) in self.globals.borrow_mut().translated_trace() {
                    write!(f, "  {}, line {}\n", source_name, lineno)?;
                    if let Some(line) = line {
                        write!(f, "    {}\n", line)?;
                    }
                }
                Ok(())
            }
        }

        Disp {
            globals: RefCell::new(self),
        }
    }

    pub fn trace_str(&mut self) -> String {
        format!("{}", self.trace_fmt())
    }

    pub fn new_exc_kind(
        &mut self,
        base: Rc<ExceptionKind>,
        name: RcStr,
        message: RcStr,
        fields: Option<Vec<RcStr>>,
    ) -> Rc<ExceptionKind> {
        self.exception_registry.add(base, name, message, fields)
    }

    pub fn builtin_functions(&self) -> &NativeFunctions {
        &self.builtin_functions
    }

    pub fn builtin_exceptions(&self) -> &BuiltinExceptions {
        &self.builtin_exceptions
    }

    pub fn exc_occurred(&self) -> bool {
        self.exc.is_some()
    }

    pub fn exc_move(&mut self) -> Exception {
        std::mem::replace(&mut self.exc, None).expect("Tried to get an error when none is present")
    }

    pub fn set_exc<T>(&mut self, exc: Exception) -> Result<T, ErrorIndicator> {
        if let Some(old_exc) = &self.exc {
            panic!(
                "New exception set before old exception was read (old = {:?}, new = {:?}",
                old_exc, exc
            );
        }
        self.exc = Some(exc);
        Err(ErrorIndicator)
    }

    pub fn set_exc_other<T>(&mut self, message: RcStr) -> Result<T, ErrorIndicator> {
        self.set_exc(Exception::new(
            self.builtin_exceptions.RuntimeError.clone(),
            vec![message.into()],
        ))
    }

    pub fn set_exc_str<T>(&mut self, message: &str) -> Result<T, ErrorIndicator> {
        self.set_exc(Exception::new(
            self.builtin_exceptions.RuntimeError.clone(),
            vec![message.into()],
        ))
    }

    fn set_escape_to_trampoline_exc<T>(&mut self) -> Result<T, ErrorIndicator> {
        self.set_exc(Exception::new(
            self.builtin_exceptions.EscapeToTrampoline.clone(),
            vec![],
        ))
    }

    pub fn set_exc_legacy<T>(&mut self, error: EvalError) -> Result<T, ErrorIndicator> {
        match error {
            EvalError::IOError(error) => self.set_io_error(error),
            _ => self.set_exc_other(format!("{}", error).into()),
        }
    }

    pub fn set_kind_error<T>(
        &mut self,
        expected: ValueKind,
        got: ValueKind,
    ) -> Result<T, ErrorIndicator> {
        self.set_exc(Exception::new(
            self.builtin_exceptions.ValueKindError.clone(),
            vec![
                format!("{:?}", expected).into(),
                format!("{:?}", got).into(),
            ],
        ))
    }

    pub fn set_name_error<T>(&mut self, name: RcStr) -> Result<T, ErrorIndicator> {
        self.set_exc(Exception::new(
            self.builtin_exceptions.NameError.clone(),
            vec![name.into()],
        ))
    }

    pub fn set_operand_type_error<T>(
        &mut self,
        operator: &str,
        operands: Vec<&Value>,
    ) -> Result<T, ErrorIndicator> {
        self.set_exc(Exception::new(
            self.builtin_exceptions.OperandTypeError.clone(),
            vec![
                operator.into(),
                Value::List(Rc::new(
                    operands
                        .iter()
                        .map(|v| Value::from(format!("{:?}", v.kind())))
                        .collect(),
                )),
            ],
        ))
    }

    pub fn set_static_attr_error<T>(
        &mut self,
        attr: Symbol,
        cls: Value,
    ) -> Result<T, ErrorIndicator> {
        self.set_exc(Exception::new(
            self.builtin_exceptions.StaticAttributeError.clone(),
            vec![attr.into(), cls],
        ))
    }

    pub fn set_key_error<T>(&mut self, message: &RcStr) -> Result<T, ErrorIndicator> {
        self.set_exc(Exception::new(
            self.builtin_exceptions.KeyError.clone(),
            vec![message.clone().into()],
        ))
    }

    pub fn set_empty_pop_error<T>(&mut self) -> Result<T, ErrorIndicator> {
        self.set_exc(Exception::new(
            self.builtin_exceptions.PopFromEmptyCollection.clone(),
            vec![],
        ))
    }

    pub fn set_assert_error<T>(&mut self, message: &RcStr) -> Result<T, ErrorIndicator> {
        self.set_exc(Exception::new(
            self.builtin_exceptions.AssertionError.clone(),
            vec![message.clone().into()],
        ))
    }

    pub fn set_os_error<T>(&mut self, message: &RcStr) -> Result<T, ErrorIndicator> {
        self.set_exc(Exception::new(
            self.builtin_exceptions.OSError.clone(),
            vec![message.clone().into()],
        ))
    }

    pub fn set_io_error<T>(&mut self, error: std::io::Error) -> Result<T, ErrorIndicator> {
        self.set_exc(Exception::new(
            self.builtin_exceptions.OSError.clone(),
            vec![format!("{:?}", error).into()],
        ))
    }

    pub fn set_utf8_error<T>(&mut self, error: std::str::Utf8Error) -> Result<T, ErrorIndicator> {
        self.set_exc(Exception::new(
            self.builtin_exceptions.OSError.clone(),
            vec![format!("{:?}", error).into()],
        ))
    }

    pub fn builtins(&self) -> &HashMap<RcStr, Value> {
        &self.builtins
    }

    pub fn builtin_classes(&self) -> &BuiltinClasses {
        &self.builtin_classes
    }

    pub fn set_custom_source_finder<F>(&mut self, f: F)
    where
        F: Fn(&str) -> Result<Option<String>, String> + 'static,
    {
        self.finder.set_custom_finder(f);
    }

    pub fn add_source_root(&mut self, root: RcPath) {
        self.finder.add_root(root);
    }

    pub fn add_source_roots_from_path_str(&mut self, roots: &str) {
        self.finder.add_roots_from_str(roots);
    }

    pub fn add_roots_from_env(&mut self) -> Result<(), std::env::VarError> {
        self.finder.add_roots_from_env()
    }

    pub fn add_embedded_source(&mut self, module_name: RcStr, data: &'static str) {
        self.finder.add_embedded_source(module_name, data);
    }

    pub fn add_native_module<F>(&mut self, name: RcStr, f: F)
    where
        F: FnOnce(&mut Globals) -> EvalResult<HMap<RcStr, Rc<RefCell<Value>>>> + 'static,
    {
        self.finder.add_native(name, f)
    }

    pub fn add_file_as_module(&mut self, name: RcStr, path: RcPath) -> Result<(), std::io::Error> {
        self.finder.add_file(name, path)?;
        Ok(())
    }

    pub fn set_cli_args(&mut self, args: Vec<RcStr>) {
        self.cli_args = args;
    }

    pub fn cli_args(&self) -> &Vec<RcStr> {
        &self.cli_args
    }

    pub fn load_by_symbol(&mut self, symbol: Symbol) -> EvalResult<Rc<Module>> {
        let name = RcStr::from(symbol);
        self.load(&name)
    }

    pub fn load_by_str(&mut self, s: &str) -> EvalResult<Rc<Module>> {
        self.load(&s.into())
    }

    pub fn load(&mut self, name: &RcStr) -> EvalResult<Rc<Module>> {
        if !self.module_registry.contains_key(name) {
            match self.finder.load(name) {
                Err(SourceFinderError::IOError(error)) => {
                    return self.set_io_error(error);
                }
                Err(SourceFinderError::SourceNotFound) => {
                    return self.set_exc_str(&format!("Module {:?} not found", name.str()));
                }
                Err(SourceFinderError::ConflictingModulePaths(paths)) => {
                    return self
                        .set_exc_str(&format!("Conflicting paths for module ({:?})", paths));
                }
                Err(SourceFinderError::Custom(message)) => {
                    return self.set_exc_str(&message);
                }
                Ok(SourceItem::Native { body }) => {
                    let map = body(self)?;
                    let map = Symbol::translate_hmap(map);
                    let module = Module::new(name.clone(), None, map);
                    self.module_registry.insert(name.clone(), module);
                }
                Ok(SourceItem::File { path, data }) => {
                    let module = self.exec_module(name.clone(), Some(path), data.into())?;
                    self.module_registry.insert(name.clone(), module);
                }
                Ok(SourceItem::Embedded { data }) => {
                    let module = self.exec_module(name.clone(), None, data.into())?;
                    self.module_registry.insert(name.clone(), module);
                }
                Ok(SourceItem::Custom { data }) => {
                    let module = self.exec_module(name.clone(), None, data.into())?;
                    self.module_registry.insert(name.clone(), module);
                }
            };
        }
        Ok(self.module_registry.get(name).unwrap().clone())
    }

    fn load_prelude(&mut self) -> EvalResult<()> {
        let module = self.load_by_str("__prelude")?;
        for (key_symbol, value) in module.map_clone() {
            let name = RcStr::from(key_symbol);
            self.builtins.insert(name, value);
        }
        Ok(())
    }

    pub fn load_main(&mut self, main_module_name: &str) -> EvalResult<()> {
        self.main_module_name = Some(main_module_name.into());
        self.load_prelude()?;
        self.load_by_str(main_module_name)?;
        Ok(())
    }

    pub fn main_module_name(&self) -> &Option<RcStr> {
        &self.main_module_name
    }

    /// convenience method that will pretty-print the stack
    /// trace and call std::process::exit(1) if an error
    /// is encountered.
    ///
    /// Will also handle trampoline requests
    ///
    pub fn exit_on_error<F>(mut self, f: F) -> ()
    where
        F: FnOnce(&mut Globals) -> EvalResult<()>,
    {
        match f(&mut self) {
            Ok(r) => r,
            Err(_) => {
                let error = self.exc_move();
                if let Some(trampoline_callback) = self.move_trampoline_callback() {
                    assert_eq!(error.kind(), &self.builtin_exceptions().EscapeToTrampoline);
                    trampoline_callback(self);
                } else {
                    eprint!("{}\n{}", error, self.trace_fmt());
                    std::process::exit(1);
                }
            }
        }
    }

    /// Print a formatted error message if an error was encountered.
    /// Returns true if an error was processed, false otherwise.
    ///
    /// This method will consume the error.
    pub fn print_if_error(&mut self) -> bool {
        if self.exc.is_some() {
            let error = self.exc_move();
            eprint!("{}\n{}", error, self.trace_fmt());
            true
        } else {
            false
        }
    }

    /// Sometimes there will be cases where the Global object needs to be moved
    /// For example, a game engine may hijack the current thread and give back
    /// limited control through callbacks.
    /// For these situations, we need to unwind all the way back to where
    /// the Global object is allocated, so that we can move it.
    /// Calling escape_to_trampoline will return an EvalResult with a JumpToTrampoline
    /// exception, and allow unwinding as far as needed.
    /// Of course, once at the allocation point, the receiver must cooperate and
    /// actually check for, and call the trampoline callback.
    /// The default entrypoint (as in entry.rs) will do this.
    ///
    /// The callback should really return '!', because the trampoline will consume
    /// the Globals instance, making it impossible to retrieve the original stack
    /// trace. However at this moment, trying to return '!' from the callback
    /// gives me an experimental warning from Rust.
    ///
    pub fn escape_to_trampoline<R, F>(&mut self, f: F) -> EvalResult<R>
    where
        F: FnOnce(Globals) + 'static,
    {
        self.trampoline_callback = Some(Box::new(f));
        self.set_escape_to_trampoline_exc()
    }

    /// Method that should be called by the host (i.e. whoever owns the Globals instance)
    /// when a JumpToTrampoline method is thrown.
    /// If the host does not take care to do this, code that depends on the trampoline
    /// mechanism may not work
    pub fn move_trampoline_callback(&mut self) -> Option<Box<dyn FnOnce(Globals)>> {
        std::mem::replace(&mut self.trampoline_callback, None)
    }

    pub(crate) fn push_new_stack(&mut self, cls: Rc<Class>) {
        let len = self.trace.len();
        self.new_stack.push((cls, len));
    }

    pub(crate) fn pop_new_stack(&mut self) {
        self.new_stack.pop().unwrap();
    }

    pub(crate) fn get_class_for_new(&self) -> Option<&Rc<Class>> {
        if let Some((cls, len)) = self.new_stack.last() {
            if *len == self.trace.len() {
                return Some(cls);
            }
        }
        None
    }

    /// The global stash allows native modules to store and share arbitrary data.
    ///
    /// To share data, you need to create a new struct and implement the Stashable
    /// trait. The stash will store exactly a single instance of each type
    /// that is stored.
    ///
    /// This means that the visibility of your data in the stash lines up with
    /// the visibility of the struct you are using. This should allow you to
    /// maintain data private to your module.
    ///
    /// As part of the Stashable trait, you will need to implement Default.
    ///
    /// The first time you call get_from_stash, Default will be used to construct
    /// the values.
    ///
    pub fn get_from_stash<S: Stashable>(&mut self) -> Rc<RefCell<S>> {
        let key = TypeId::of::<S>();
        if !self.stash.contains_key(&key) {
            let typed_rc: Rc<RefCell<S>> = Rc::new(RefCell::new(S::default()));
            let untyped_rc: Rc<dyn Any> = typed_rc;
            self.stash.insert(key.clone(), untyped_rc);
        }
        let untyped_rc = self.stash.get(&key).unwrap().clone();
        let typed_rc: Rc<RefCell<S>> = untyped_rc.downcast().unwrap();
        typed_rc
    }

    /// Initializes a REPL scope with builtins for use with exec_repl
    pub fn new_repl_scope(&mut self) -> ReplScope {
        self.load_prelude().unwrap();
        let mut map = HashMap::new();
        for (key, val) in self.builtins.iter() {
            map.insert(key.clone(), Rc::new(RefCell::new(val.clone())));
        }
        map
    }

    fn exec_repl_with_ast(
        &mut self,
        scope: &mut ReplScope,
        expr: &Expression,
    ) -> EvalResult<Value> {
        let code = match compile(crate::REPL_PSEUDO_MODULE_NAME.into(), expr) {
            Ok(code) => code,
            Err(error) => {
                let (name, lineno, kind) = error.move_();
                self.trace_push(name, lineno);
                return self.set_exc_legacy(EvalError::CompileError(kind));
            }
        };

        let mut frame = match Frame::for_repl(&code, scope) {
            Ok(x) => x,
            Err(FrameError::MissingBuiltin {
                name: varname,
                lineno,
            }) => {
                self.trace_push(crate::REPL_PSEUDO_MODULE_NAME.into(), lineno);
                return self.set_name_error(varname);
            }
        };

        code.run(self, &mut frame)
    }

    /// Convenience method for determining whether some input source
    /// is ready to be passed to the exec_repl.
    /// Mainly, this is to check for unterminated grouping symbols.
    pub fn is_ready_for_repl(&self, line: &str) -> bool {
        match self.lex(line) {
            Ok(_) => true,
            Err(error) => match error.kind() {
                LexErrorKind::UnmatchedOpeningSymbol => false,
                _ => true,
            },
        }
    }

    pub fn run_repl<D: ReplDelegate + ?Sized>(self, delegate: &mut D) {
        crate::run_repl(self, delegate)
    }

    fn exec_module_with_ast(
        &mut self,
        name: RcStr,
        filename: Option<RcPath>,
        expr: &Expression,
    ) -> EvalResult<Rc<Module>> {
        let code = match compile(name.clone(), expr) {
            Ok(code) => code,
            Err(error) => {
                let (name, lineno, kind) = error.move_();
                self.trace_push(name, lineno);
                return self.set_exc_legacy(EvalError::CompileError(kind));
            }
        };
        let (mut frame, module) = match Frame::for_module(&code, filename, self.builtins()) {
            Ok(x) => x,
            Err(FrameError::MissingBuiltin {
                name: varname,
                lineno,
            }) => {
                self.trace_push(name, lineno);
                return self.set_name_error(varname);
            }
        };

        // TODO: return an Err instead if this assertion fails
        assert!(self.module_registry.insert(name, module.clone()).is_none());

        code.run(self, &mut frame)?;
        Ok(module)
    }

    pub fn exec_module(
        &mut self,
        name: RcStr,
        filename: Option<RcPath>,
        code: RcStr,
    ) -> EvalResult<Rc<Module>> {
        if let Some(filename) = filename.clone() {
            assert!(self
                .filepath_registry
                .insert(name.clone(), filename)
                .is_none());
        }
        assert!(self
            .source_registry
            .insert(name.clone(), code.clone())
            .is_none());
        let expr = self.parse(name.clone(), &*code)?;
        self.exec_module_with_ast(name, filename, &expr)
    }

    /// Execute a snippet of code as though you were running them in the REPL.
    /// There is one shared REPL scope per Globals instance.
    pub fn exec_repl(&mut self, scope: &mut ReplScope, code: &str) -> EvalResult<Value> {
        let expr = self.parse(crate::REPL_PSEUDO_MODULE_NAME.into(), code.into())?;
        self.exec_repl_with_ast(scope, &expr)
    }

    pub fn lex<'a>(&self, s: &'a str) -> Result<(Vec<Token<'a>>, Vec<(usize, usize)>), LexError> {
        self.lexer.lex(s)
    }

    pub fn parse(&mut self, name: RcStr, s: &str) -> EvalResult<Expression> {
        let (tokens, posinfo) = match self.lexer.lex(s) {
            Ok(pair) => pair,
            Err(error) => {
                let (_offset, lineno, kind) = error.move_();
                self.trace_push(name, lineno);
                let error = EvalError::LexError(kind);
                return self.set_exc_legacy(error);
            }
        };

        match self.parser.parse_tokens(tokens, posinfo) {
            Ok(expr) => Ok(expr),
            Err(error) => {
                let (_offset, lineno, kind) = error.move_();
                self.trace_push(name, lineno);
                let error = EvalError::ParseError(kind);
                return self.set_exc_legacy(error);
            }
        }
    }

    pub fn set_handle_class<T: Any>(&mut self, cls: Rc<Class>) -> EvalResult<()> {
        let key = TypeId::of::<T>();
        match self.handle_class_map.entry(key) {
            std::collections::hash_map::Entry::Occupied(_) => self.set_exc_str(&format!(
                "Class for conversion to native type {:?} already set",
                std::any::type_name::<T>()
            )),
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(cls);
                Ok(())
            }
        }
    }

    pub fn get_handle_class<T: Any>(&self) -> Option<&Rc<Class>> {
        self.handle_class_map.get(&TypeId::of::<T>())
    }

    pub fn new_handle<T: Any>(&mut self, t: T) -> EvalResult<Handle<T>> {
        if let Some(cls) = self.get_handle_class::<T>() {
            Ok(Handle::new(t, cls.clone()))
        } else {
            self.set_exc_str(&format!(
                "Class for conversion from native type {:?} not set",
                std::any::type_name::<T>()
            ))
        }
    }

    /// Convenience method for creating a new native class for use
    /// with a Handle.
    pub fn new_class<N>(
        &mut self,
        name: N,
        bases: Vec<Rc<Class>>,
        map: HashMap<Symbol, Value>,
        static_map: HashMap<Symbol, Value>,
    ) -> EvalResult<Rc<Class>>
    where
        N: Into<RcStr>,
    {
        Ok(Class::new(
            self,
            ClassKind::NativeClass,
            name.into(),
            bases,
            None,
            None,
            map,
            static_map,
        )?)
    }

    /// Convenience wrapper around the 'new_class' method
    pub fn new_class0<N>(&mut self, name: N, methods: Vec<NativeFunction>) -> EvalResult<Rc<Class>>
    where
        N: Into<RcStr>,
    {
        let mut map = HashMap::new();
        for method in methods {
            let name = Symbol::from(method.name());
            map.insert(name, method.into());
        }
        self.new_class(name, vec![], map, HashMap::new())
    }
}
