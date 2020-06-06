mod builder;
mod opc;

use crate::ArgumentError;
use crate::ErrorIndicator;
use crate::EvalError;
use crate::EvalResult;
use crate::Globals;
use crate::HMap;
use crate::Module;
use crate::ParameterInfo;
use crate::RcPath;
use crate::RcStr;
use crate::Symbol;
use crate::SymbolRegistryHandle;
use crate::Table;
use crate::Value;
use opc::OpcodeArgumentType;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;

use std::rc::Rc;

pub use builder::CodeBuilder;
pub use builder::CodeBuilderError;

pub enum CodeKind {
    Module,
    Function,
    Generator,
}

pub struct Code {
    kind: CodeKind,
    code: Vec<usize>,
    names: Vec<Symbol>,      // used names used (e.g. keyword args, parameters)
    constants: Vec<Value>,   // constants used in the code
    children: Vec<Rc<Code>>, // code objects for nested functions
    locals: Vec<Symbol>,     // local variable names
    freevars: Vec<Symbol>,   // free variable names
    ocellvars: Vec<Symbol>,  // owned cell variable names (i.e. excludes freevars)

    // Owned cell variables are variables that would be local, but appear as
    // a free variable in a nested function
    // Free variables and owned cell variables together form the cell variables
    // of a function

    // parameter info has to be a part of the Code object because
    // the details of how arguments get assigned to variables can
    // vary depending on the internal state of Code (e.g. is the argument
    // a cellvar?)
    // Of course this isn't very meaningful when dealing with modules since
    // modules don't take arguments
    parameter_info: ParameterInfo,
    argument_map: ArgumentMap,

    // contains the function name if this is a function, or the
    // module name, if this is a module
    // This isn't strictly necessary, but if isn't provided here,
    // the name of the function would have to be explicitly loaded
    // on that stack when functions are created
    full_name: RcStr,
    short_name: RcStr,

    // debug information
    module_name: RcStr,          // name of the module this code object is from
    lineno: usize,               // line in the file that this code object starts
    lnotab: Vec<(usize, usize)>, // line number table containing (opcode-offset, lineno) pairs

    doc: Option<RcStr>,
}

impl fmt::Debug for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<code object {:?}:{}>", self.full_name, self.lineno)
    }
}

pub enum GeneratorResult {
    Done(Value),
    Yield(Value),
    Error,
}

impl Code {
    pub fn is_generator(&self) -> bool {
        if let CodeKind::Generator = self.kind {
            true
        } else {
            false
        }
    }

    pub fn full_name(&self) -> &RcStr {
        &self.full_name
    }

    pub fn short_name(&self) -> &RcStr {
        &self.short_name
    }

    pub fn module_name(&self) -> &RcStr {
        &self.module_name
    }

    pub fn freevars(&self) -> &Vec<Symbol> {
        &self.freevars
    }

    fn find_var(&self, varname: Symbol) -> Option<VariableLocation> {
        for (i, name) in self.locals.iter().enumerate() {
            if varname == *name {
                return Some(VariableLocation::Local(i));
            }
        }
        for (i, name) in self.freevars.iter().enumerate() {
            if varname == *name {
                return Some(VariableLocation::Cell(i));
            }
        }
        for (i, name) in self.ocellvars.iter().enumerate() {
            if varname == *name {
                return Some(VariableLocation::Cell(self.freevars.len() + i));
            }
        }
        None
    }

    /// Find the line number of the location where the given variable is first used
    pub fn find_var_first_used_lineno(&self, varname: Symbol) -> Option<usize> {
        let loc = self.find_var(varname)?;
        for (pos, opcode, args) in self.parsed_opcodes() {
            // We ignore LOAD_CELL opcodes, since this is an indication that
            // the variable is being used in a nested function, and isn't likely
            // to be as nice to look at
            if opcode == opc::LOAD_CELL {
                continue;
            }
            let info = &opc::OPCODE_INFO_MAP[opcode];
            for (argtype, arg) in info.argtypes().iter().zip(args) {
                if argtype.as_var_loc(*arg) == Some(loc) {
                    return Some(self.find_lineno_for_opcode_at(pos));
                } else if *argtype == OpcodeArgumentType::Code {
                    // If a nested function uses this variable, and we haven't
                    // seen any other usages yet, this is probably what we want to show
                    let child = &self.children[*arg];
                    if child.freevars.contains(&varname) {
                        return child.find_var_first_used_lineno(varname);
                    }
                }
            }
        }
        None
    }

    fn find_lineno_for_opcode_at(&self, i: usize) -> usize {
        let lnotab = &self.lnotab;
        let mut lower = 0;
        let mut upper = lnotab.len();
        let mut best = lnotab.get(0).map(|(_, ln)| *ln).unwrap_or(0);
        while lower + 1 < upper {
            let mid = (lower + upper) / 2;
            if lnotab[mid].0 <= i {
                best = lnotab[mid].1;
                lower = mid;
            } else {
                upper = mid;
            }
        }
        best
    }

    pub fn parsed_opcodes(&self) -> ParsedOpcodesIter {
        ParsedOpcodesIter {
            code: &self.code,
            i: 0,
        }
    }

    pub fn parameter_info(&self) -> &ParameterInfo {
        &self.parameter_info
    }

    pub fn assign_args(
        &self,
        frame: &mut Frame,
        args: Vec<Value>,
        kwargs: Option<HashMap<Symbol, Value>>,
    ) -> Result<(), ArgumentError> {
        let (args, kwargs) = self.parameter_info.translate(args, kwargs)?;
        self.argument_map.apply(frame, args, kwargs);
        Ok(())
    }

    pub fn run(&self, globals: &mut Globals, frame: &mut Frame) -> EvalResult<Value> {
        while frame.i < self.code.len() {
            match opc::step(globals, frame, self) {
                Ok(()) => (),
                Err(opc::StepException::Error) => return Err(ErrorIndicator),
                Err(opc::StepException::Yield) => {
                    return globals.set_exc_legacy(EvalError::YieldOutsideGenerator)
                }
                Err(opc::StepException::Return) => {
                    return Ok(frame.stack.pop().unwrap());
                }
            }
        }
        assert_eq!(frame.stack.len(), 1);
        Ok(frame.stack.pop().unwrap())
    }

    /// Assumes this code object is for a generator object, and starts the generator
    pub fn start(&self, globals: &mut Globals, frame: &mut Frame) -> GeneratorResult {
        self.resume0(globals, frame)
    }

    /// Assumes this code object is a generator, and resumes the generator with the given value
    pub fn resume(
        &self,
        globals: &mut Globals,
        frame: &mut Frame,
        value: Value,
    ) -> GeneratorResult {
        frame.stack.push(value);
        self.resume0(globals, frame)
    }

    fn resume0(&self, globals: &mut Globals, frame: &mut Frame) -> GeneratorResult {
        while frame.i < self.code.len() {
            match opc::step(globals, frame, self) {
                Ok(()) => (),
                Err(opc::StepException::Error) => return GeneratorResult::Error,
                Err(opc::StepException::Yield) => {
                    return GeneratorResult::Yield(frame.stack.pop().unwrap())
                }
                Err(opc::StepException::Return) => {
                    return GeneratorResult::Done(frame.stack.pop().unwrap());
                }
            }
        }
        assert_eq!(frame.stack.len(), 1);
        GeneratorResult::Done(frame.stack.pop().unwrap())
    }

    pub fn doc(&self) -> &Option<RcStr> {
        &self.doc
    }

    pub fn debugstr(&self) -> String {
        let mut ret = self.debugstr0();
        for child in &self.children {
            ret.push('\n');
            ret.push_str(&child.debugstr());
        }
        ret
    }

    /// Dumps the "disassembly" of this Code object, not including
    /// any child code objects
    pub fn debugstr0(&self) -> String {
        let lmap = {
            let mut map = HashMap::new();
            for (offset, lineno) in &self.lnotab {
                map.insert(*offset, *lineno);
            }
            map
        };
        let map = opc::OPCODE_INFO_MAP;
        let mut ret = String::new();
        let mut i = 0; // index into self.code
        ret.push_str(&format!("Code object {}\n", self.full_name));
        while i < self.code.len() {
            let opcode = self.code[i];
            let info = &map[opcode];
            let ln = if let Some(lineno) = lmap.get(&i) {
                lineno.to_string()
            } else {
                "".to_owned()
            };
            ret.push_str(&format!("{:>6} {:>6} {}", ln, i, info.name()));

            i += 1;

            let mut first = true;
            for argtype in info.argtypes() {
                if first {
                    ret.push_str(&format!(" "));
                } else {
                    ret.push_str(&format!(", "));
                }
                let arg = self.code[i];
                ret.push_str(&format!("{}", arg));
                let argtsr = match argtype {
                    OpcodeArgumentType::Int => "".to_owned(),
                    OpcodeArgumentType::Label => "".to_owned(),
                    OpcodeArgumentType::Code => {
                        format!(" (Code object {})", self.children[arg].full_name())
                    }
                    OpcodeArgumentType::Local => format!(" ({})", self.locals[arg]),
                    OpcodeArgumentType::Cell => {
                        let (name, kind) = if arg < self.freevars.len() {
                            (&self.freevars[arg], " (free)")
                        } else {
                            (&self.ocellvars[arg - self.freevars.len()], "")
                        };
                        format!(" ({}{})", name, kind)
                    }
                    OpcodeArgumentType::Const => {
                        let value = &self.constants[arg];
                        format!(" ({:?})", value)
                    }
                    OpcodeArgumentType::Name => format!(" ({})", self.names[arg]),
                    OpcodeArgumentType::LineNumber => format!(""),
                };
                ret.push_str(&argtsr);
                first = false;
                i += 1;
            }

            ret.push_str("\n");
        }
        ret
    }
}

pub struct ParsedOpcodesIter<'a> {
    code: &'a [usize],
    i: usize,
}

impl<'a> Iterator for ParsedOpcodesIter<'a> {
    type Item = (usize, usize, &'a [usize]);

    fn next(&mut self) -> Option<Self::Item> {
        if self.i < self.code.len() {
            let i = self.i;
            let op = self.code[self.i];
            let info = &opc::OPCODE_INFO_MAP[op];
            self.i += 1 + info.argtypes().len();
            let args = &self.code[i + 1..self.i];
            Some((i, op, args))
        } else {
            None
        }
    }
}

pub struct Frame {
    i: usize,                          // index into opcode table
    stack: Vec<Value>,                 //operand stack
    locals: Vec<Value>,                // local variables
    cellvars: Vec<Rc<RefCell<Value>>>, // cellvars = free and owned cell variables
}

impl Frame {
    fn new(nlocals: usize, cellvars: Vec<Rc<RefCell<Value>>>) -> Frame {
        Frame {
            i: 0,
            stack: Vec::new(),
            locals: {
                let mut locals = Vec::new();
                for _ in 0..nlocals {
                    locals.push(Value::Uninitialized);
                }
                locals
            },
            cellvars,
        }
    }

    /// Creates a Frame for executing a function
    pub fn for_func(code: &Code, freevar_bindings: Vec<Rc<RefCell<Value>>>) -> Frame {
        // nlocals = number of local variables
        // freevar_bindings = references to cell values for the function's free variables
        // nocellvars = the number of owned cell variables
        let nocellvars = code.ocellvars.len();
        let cellvars = {
            let mut cellvars = freevar_bindings;
            for _ in 0..nocellvars {
                cellvars.push(Rc::new(RefCell::new(Value::Uninitialized)));
            }
            cellvars
        };
        Self::new(code.locals.len(), cellvars)
    }

    /// Creates a Frame for executing a module.
    /// This function also returns a Module instance "bound" to this Frame.
    /// Executing the returned Frame will populate the associated Module.
    pub fn for_module(
        symbol_registry: SymbolRegistryHandle,
        code: &Code,
        filename: Option<RcPath>,
        builtins: &HashMap<RcStr, Value>,
    ) -> Result<(Frame, Rc<Module>), FrameError> {
        let mut cellvars = Vec::new();

        // Get all the builtin values that the module needs
        // and throw an error if the variable could not be found
        for name in &code.freevars {
            let name_rcstr = symbol_registry.rcstr(*name).clone();
            if let Some(value) = builtins.get(&name_rcstr) {
                cellvars.push(Rc::new(RefCell::new(value.clone())));
            } else {
                cellvars.push(Rc::new(RefCell::new(match name.str() {
                    "__name" => Value::String(code.full_name().clone()),
                    "__file" => match filename.clone() {
                        Some(path) => Value::Path(path),
                        None => Value::Nil,
                    },
                    _ => {
                        return Err(FrameError::MissingBuiltin {
                            name: name_rcstr,
                            lineno: code.find_var_first_used_lineno(*name).unwrap(),
                        });
                    }
                })));
            }
        }

        // create the shared owned-cell variables
        let mut module_map = HMap::new();
        for name in &code.ocellvars {
            let cell = Rc::new(RefCell::new(Value::Uninitialized));
            module_map.insert(name.clone(), cell.clone());
            cellvars.push(cell);
        }

        let frame = Self::new(code.locals.len(), cellvars);
        let module = Module::new(code.full_name.clone(), code.doc().clone(), module_map);
        Ok((frame, module))
    }
}

#[derive(Debug)]
pub enum FrameError {
    MissingBuiltin { name: RcStr, lineno: usize },
}

#[derive(Clone, Copy, PartialEq)]
enum VariableLocation {
    Local(usize),
    Cell(usize),
}

impl VariableLocation {
    fn apply(&self, frame: &mut Frame, value: Value) {
        match self {
            VariableLocation::Local(i) => frame.locals[*i] = value,
            VariableLocation::Cell(i) => *frame.cellvars[*i].borrow_mut() = value,
        }
    }
}

struct ArgumentMap {
    positional: Vec<VariableLocation>,
    variadic: Option<VariableLocation>,
    kwparam: Option<VariableLocation>,
}

impl ArgumentMap {
    /// Constructs an ArgumentMap given some information about the
    /// corresponding Code object.
    /// Will panic when some assumptions don't hold (e.g. all names in
    /// parameter_info must appear in locals, freevars, or ocellvars)
    fn new(parameter_info: &ParameterInfo, map: &HashMap<Symbol, VariableLocation>) -> ArgumentMap {
        let mut positional = Vec::new();
        for name in parameter_info.required() {
            positional.push(*map.get(name).unwrap());
        }
        for (name, _) in parameter_info.optional() {
            positional.push(*map.get(name).unwrap());
        }
        let variadic = if let Some(name) = parameter_info.variadic() {
            Some(*map.get(name).unwrap())
        } else {
            None
        };
        let kwparam = if let Some(name) = parameter_info.keywords() {
            Some(*map.get(name).unwrap())
        } else {
            None
        };

        ArgumentMap {
            positional,
            variadic,
            kwparam,
        }
    }

    // We don't return any errors here, because we assume that all checks
    // are already done on the arguments with the ParameterInfo in the
    // corresponding Code object
    fn apply(&self, frame: &mut Frame, args: Vec<Value>, kwargs: Option<HashMap<Symbol, Value>>) {
        let mut args = args.into_iter();
        for dest in &self.positional {
            dest.apply(frame, args.next().unwrap());
        }
        if let Some(variadic) = &self.variadic {
            let rest: Vec<Value> = args.collect();
            let value = Value::List(rest.into());
            variadic.apply(frame, value);
        }
        if let Some(kwparam) = &self.kwparam {
            let kwargs = if let Some(kwargs) = kwargs {
                kwargs
            } else {
                HashMap::new()
            };
            kwparam.apply(frame, Value::Table(Table::new(kwargs).into()));
        }
    }
}
