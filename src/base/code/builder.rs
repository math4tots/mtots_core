use super::opc;
use super::ArgumentMap;
use super::Symbol;
use super::VariableLocation;
use crate::short_name_from_full_name;
use crate::Binop;
use crate::ClassKind;
use crate::Code;
use crate::CodeKind;
use crate::ConstValue;
use crate::ParameterInfo;
use crate::RcStr;
use crate::SymbolRegistryHandle;
use crate::Unop;
use crate::Value;
use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Label(usize);

#[allow(dead_code)]
enum PseudoOpcode {
    LineNumber(usize), // Indicates line number for following opcodes
    Label(Label),      // target of a jump location
    Pop,
    RotTwo,
    SwapTos1Tos2,
    PullTos2,
    PullTos3,
    DupTop,
    LoadConst(usize),
    LoadVar(RcStr),
    StoreVar(RcStr),
    LoadCell(RcStr),
    LoadDunderNew,
    LoadClassForNew,
    Nonlocal(RcStr),
    Unpack(usize),
    MakeMutableString(usize),
    MakeList(usize),
    MakeTable(usize),
    MakeMap(usize),
    MakeMutableList(usize),
    MakeMutableMap(usize),
    LoadMethod(usize),
    CallFunction(usize, usize),
    CallFunctionGeneric(usize),
    ExtendList(usize),
    ExtendTable(usize),
    MakeFunction(usize),
    MakeClass(usize, usize),
    MakeExceptionKind(usize),
    Jump(Label),
    PopJumpIfTrue(Label),
    PopJumpIfFalse(Label),
    JumpIfTrueOrPop(Label),
    JumpIfFalseOrPop(Label),
    GetIter,
    ForIter(Label),
    Binop(usize, Binop),
    Unop(usize, Unop),
    LoadAttribute(usize),
    StoreAttribute(usize),
    LoadStaticAttribute(usize),
    Import(usize, usize),
    Yield,
    Return,
    Breakpoint,
}

pub struct CodeBuilder {
    symbol_registry: SymbolRegistryHandle,
    kind: CodeKind,
    parameter_info: ParameterInfo,
    module_name: RcStr,
    full_name: RcStr,
    lineno: usize,
    code: Vec<PseudoOpcode>,
    names: Vec<RcStr>,
    name_map: HashMap<RcStr, usize>,
    constants: Vec<Value>,
    children: Vec<Rc<Code>>,
    constmap: HashMap<ConstValue, usize>,
    label_count: usize,
    current_lineno: usize,
    doc: Option<RcStr>,
}

pub type CodeBuilderResult<T> = Result<T, CodeBuilderError>;
#[derive(Debug)]
pub enum CodeBuilderError {}
impl CodeBuilder {
    pub fn new(
        symbol_registry: SymbolRegistryHandle,
        kind: CodeKind,
        parameter_info: ParameterInfo,
        module_name: RcStr,
        full_name: RcStr,
        lineno: usize,
        doc: Option<RcStr>,
    ) -> CodeBuilder {
        let mut cb = CodeBuilder {
            symbol_registry,
            kind,
            parameter_info,
            module_name,
            full_name,
            lineno,
            code: Vec::new(),
            names: Vec::new(),
            name_map: HashMap::new(),
            constants: Vec::new(),
            children: Vec::new(),
            constmap: HashMap::new(),
            label_count: 0,
            current_lineno: lineno,
            doc,
        };
        // If the outermost expression is not a block,
        // there might not be an explicit lineno marker
        // at the beginning. The call below ensures that
        // the line number at the beginning defaults to
        // the starting line number of the code object.
        cb.lineno(lineno);
        cb
    }

    pub fn kind(&self) -> CodeKind {
        self.kind
    }

    pub fn symbol_registry(&self) -> &SymbolRegistryHandle {
        &self.symbol_registry
    }

    pub fn intern_rcstr(&self, s: &RcStr) -> Symbol {
        self.symbol_registry.intern_rcstr(s)
    }

    pub fn module_name(&self) -> &RcStr {
        &self.module_name
    }

    pub fn full_name(&self) -> &RcStr {
        &self.full_name
    }

    pub fn for_module(
        symbol_registry: SymbolRegistryHandle,
        name: RcStr,
        doc: Option<RcStr>,
    ) -> CodeBuilder {
        Self::new(
            symbol_registry,
            CodeKind::Module,
            ParameterInfo::empty(),
            name.clone(),
            name,
            1,
            doc,
        )
    }

    pub fn for_func(
        symbol_registry: SymbolRegistryHandle,
        parameter_info: ParameterInfo,
        module_name: RcStr,
        full_name: RcStr,
        lineno: usize,
        doc: Option<RcStr>,
    ) -> CodeBuilder {
        Self::new(
            symbol_registry,
            CodeKind::Function,
            parameter_info,
            module_name,
            full_name,
            lineno,
            doc,
        )
    }

    pub fn for_generator(
        symbol_registry: SymbolRegistryHandle,
        parameter_info: ParameterInfo,
        module_name: RcStr,
        full_name: RcStr,
        lineno: usize,
        doc: Option<RcStr>,
    ) -> CodeBuilder {
        Self::new(
            symbol_registry,
            CodeKind::Generator,
            parameter_info,
            module_name,
            full_name,
            lineno,
            doc,
        )
    }

    pub fn lineno(&mut self, lineno: usize) {
        if let Some(PseudoOpcode::LineNumber(_)) = self.code.last() {
            self.code.pop();
        }
        self.code.push(PseudoOpcode::LineNumber(lineno));
        self.current_lineno = lineno;
    }

    pub fn pop(&mut self) {
        self.code.push(PseudoOpcode::Pop);
    }

    pub fn rot_two(&mut self) {
        self.code.push(PseudoOpcode::RotTwo);
    }

    #[allow(dead_code)]
    pub fn swap_tos1_tos2(&mut self) {
        self.code.push(PseudoOpcode::SwapTos1Tos2);
    }

    pub fn pull_tos2(&mut self) {
        self.code.push(PseudoOpcode::PullTos2);
    }

    pub fn pull_tos3(&mut self) {
        self.code.push(PseudoOpcode::PullTos3);
    }

    pub fn dup_top(&mut self) {
        self.code.push(PseudoOpcode::DupTop);
    }

    fn add_to_names(&mut self, name: &RcStr) -> usize {
        if let Some(index) = self.name_map.get(name) {
            *index
        } else {
            let index = self.names.len();
            self.names.push(name.clone());
            self.name_map.insert(name.clone(), index);
            index
        }
    }

    fn add_to_constants(&mut self, value: Value) -> usize {
        let index = self.constants.len();
        self.constants.push(value);
        index
    }

    pub fn load_const<CV: Into<ConstValue>>(&mut self, cv: CV) {
        let cv = cv.into();
        let index = if let Some(index) = self.constmap.get(&cv) {
            *index
        } else {
            let index = self.add_to_constants(cv.clone().into());
            self.constmap.insert(cv, index);
            index
        };
        self.code.push(PseudoOpcode::LoadConst(index));
    }

    pub fn make_mutable_string(&mut self, string: &RcStr) {
        let namei = self.add_to_names(string);
        self.code.push(PseudoOpcode::MakeMutableString(namei));
    }

    pub fn add_code_obj(&mut self, code: Rc<Code>) -> usize {
        let index = self.children.len();
        self.children.push(code);
        index
    }

    pub fn load_var(&mut self, name: RcStr) {
        self.code.push(PseudoOpcode::LoadVar(name));
    }

    pub fn store_var(&mut self, name: RcStr) {
        self.code.push(PseudoOpcode::StoreVar(name));
    }

    pub fn load_cell(&mut self, name: RcStr) {
        self.code.push(PseudoOpcode::LoadCell(name));
    }

    pub fn load_dunder_new(&mut self) {
        self.code.push(PseudoOpcode::LoadDunderNew);
    }

    pub fn load_class_for_new(&mut self) {
        self.code.push(PseudoOpcode::LoadClassForNew);
    }

    pub fn nonlocal(&mut self, name: RcStr) {
        self.code.push(PseudoOpcode::Nonlocal(name));
    }

    pub fn unpack(&mut self, n: usize) {
        self.code.push(PseudoOpcode::Unpack(n));
    }

    pub fn make_list(&mut self, argc: usize) {
        self.code.push(PseudoOpcode::MakeList(argc));
    }

    pub fn make_table(&mut self, argc: usize) {
        self.code.push(PseudoOpcode::MakeTable(argc));
    }

    pub fn make_map(&mut self, argc: usize) {
        self.code.push(PseudoOpcode::MakeMap(argc));
    }

    pub fn make_mutable_list(&mut self, argc: usize) {
        self.code.push(PseudoOpcode::MakeMutableList(argc));
    }

    pub fn make_mutable_map(&mut self, argc: usize) {
        self.code.push(PseudoOpcode::MakeMutableMap(argc));
    }

    pub fn load_method(&mut self, name: &RcStr) {
        let namei = self.add_to_names(name);
        self.code.push(PseudoOpcode::LoadMethod(namei));
    }

    pub fn call_func(&mut self, argc: usize) {
        self.code
            .push(PseudoOpcode::CallFunction(self.current_lineno, argc));
    }

    pub fn call_func_generic(&mut self) {
        self.code
            .push(PseudoOpcode::CallFunctionGeneric(self.current_lineno));
    }

    pub fn extend_list(&mut self) {
        self.code
            .push(PseudoOpcode::ExtendList(self.current_lineno))
    }

    pub fn extend_table(&mut self) {
        self.code
            .push(PseudoOpcode::ExtendTable(self.current_lineno))
    }

    pub fn make_func(&mut self, i: usize) {
        self.code.push(PseudoOpcode::MakeFunction(i));
    }

    pub fn make_class(&mut self, full_name: &RcStr, class_kind: ClassKind) {
        let full_name = self.add_to_names(full_name);
        self.code
            .push(PseudoOpcode::MakeClass(full_name, class_kind.to_usize()));
    }

    pub fn make_exception_kind(&mut self, full_name: &RcStr) {
        let full_name = self.add_to_names(full_name);
        self.code.push(PseudoOpcode::MakeExceptionKind(full_name));
    }

    pub fn new_label(&mut self) -> Label {
        let label_id = self.label_count;
        self.label_count += 1;
        Label(label_id)
    }

    pub fn label(&mut self, label: Label) {
        self.code.push(PseudoOpcode::Label(label));
    }

    pub fn jump(&mut self, label: Label) {
        self.code.push(PseudoOpcode::Jump(label));
    }

    #[allow(dead_code)]
    pub fn pop_jump_if_true(&mut self, label: Label) {
        self.code.push(PseudoOpcode::PopJumpIfTrue(label));
    }

    pub fn pop_jump_if_false(&mut self, label: Label) {
        self.code.push(PseudoOpcode::PopJumpIfFalse(label));
    }

    pub fn jump_if_true_or_pop(&mut self, label: Label) {
        self.code.push(PseudoOpcode::JumpIfTrueOrPop(label));
    }

    pub fn jump_if_false_or_pop(&mut self, label: Label) {
        self.code.push(PseudoOpcode::JumpIfFalseOrPop(label));
    }

    pub fn get_iter(&mut self) {
        self.code.push(PseudoOpcode::GetIter);
    }

    pub fn for_iter(&mut self, label: Label) {
        self.code.push(PseudoOpcode::ForIter(label));
    }

    pub fn binop(&mut self, op: Binop) {
        self.code.push(PseudoOpcode::Binop(self.current_lineno, op));
    }

    pub fn unop(&mut self, op: Unop) {
        self.code.push(PseudoOpcode::Unop(self.current_lineno, op));
    }

    pub fn load_attr(&mut self, name: &RcStr) {
        let i = self.add_to_names(name);
        self.code.push(PseudoOpcode::LoadAttribute(i));
    }

    pub fn load_static_attr(&mut self, name: &RcStr) {
        let i = self.add_to_names(name);
        self.code.push(PseudoOpcode::LoadStaticAttribute(i));
    }

    pub fn store_attr(&mut self, name: &RcStr) {
        let i = self.add_to_names(name);
        self.code.push(PseudoOpcode::StoreAttribute(i));
    }

    pub fn import_(&mut self, name: &RcStr) {
        let i = self.add_to_names(name);
        self.code.push(PseudoOpcode::Import(self.current_lineno, i));
    }

    pub fn yield_(&mut self) {
        self.code.push(PseudoOpcode::Yield);
    }

    pub fn return_(&mut self) {
        self.code.push(PseudoOpcode::Return);
    }

    pub fn breakpoint(&mut self) {
        self.code.push(PseudoOpcode::Breakpoint);
    }

    pub fn build(self) -> CodeBuilderResult<Code> {
        // pass 1: find all used and assigned variables in this scope
        // include freevars of nested functions in set of used variables.
        // allvars contains a list of all variables that appear in order
        // of their appearance
        let (allvars, assignedvars, inner_freevars, nonlocals) = {
            struct State {
                allvars: Vec<Symbol>,
                allvars_set: HashSet<Symbol>,
                assignedvars: HashSet<Symbol>,
                inner_freevars: HashSet<Symbol>,
                nonlocals: HashSet<Symbol>,
            }
            impl State {
                fn read(&mut self, name: Symbol) {
                    if self.allvars_set.insert(name) {
                        self.allvars.push(name);
                        self.allvars_set.insert(name);
                    }
                }
                fn write(&mut self, name: Symbol) {
                    self.read(name);
                    self.assignedvars.insert(name);
                }
                fn inner_freevar(&mut self, name: Symbol) {
                    self.read(name);
                    self.inner_freevars.insert(name);
                }
                fn nonlocal(&mut self, name: Symbol) {
                    self.read(name);
                    self.nonlocals.insert(name);
                }
            }
            let mut state = State {
                allvars: Vec::new(),
                allvars_set: HashSet::new(),
                assignedvars: HashSet::new(),
                inner_freevars: HashSet::new(),
                nonlocals: HashSet::new(),
            };
            for name in self.parameter_info.required() {
                state.write(*name);
            }
            for (name, _) in self.parameter_info.optional() {
                state.write(*name);
            }
            if let Some(name) = self.parameter_info.variadic() {
                state.write(*name);
            }
            if let Some(name) = self.parameter_info.keywords() {
                state.write(*name);
            }
            for opcode in &self.code {
                match opcode {
                    PseudoOpcode::StoreVar(name) => {
                        state.write(self.intern_rcstr(name));
                    }
                    PseudoOpcode::LoadVar(name) => {
                        state.read(self.intern_rcstr(name));
                    }
                    PseudoOpcode::LoadCell(name) => {
                        state.inner_freevar(self.intern_rcstr(name));
                    }
                    PseudoOpcode::Nonlocal(name) => {
                        state.nonlocal(self.intern_rcstr(name));
                    }
                    _ => (),
                }
            }
            // If this is a module, we should have no true local variables
            // All local variables should live in cells so that they are accessible
            // from the module object
            if let CodeKind::Module = self.kind {
                for name in &state.allvars {
                    state.inner_freevars.insert(name.clone());
                }
            }
            (
                state.allvars,
                state.assignedvars,
                state.inner_freevars,
                state.nonlocals,
            )
        };

        // pass 2: categorize all variables as either local, free or owned-cell
        let (locals, freevars, ocellvars) = {
            let mut locals = Vec::new();
            let mut freevars = Vec::new();
            let mut ocellvars = Vec::new();
            for var in allvars {
                if assignedvars.contains(&var) && !nonlocals.contains(&var) {
                    if inner_freevars.contains(&var) {
                        // If a variable is assigned to, and a nested function
                        // needs it as a freevar, it's an owned-cell variable
                        ocellvars.push(var);
                    } else {
                        // If a variable is assigned to, and is not used in
                        // in any nested functions, it's a true local variable
                        locals.push(var);
                    }
                } else {
                    // If a variable is used but never assigned to,
                    // (or marked non-local)
                    // it's a free variable
                    freevars.push(var);
                }
            }
            (locals, freevars, ocellvars)
        };

        // From the variable information we just computed,
        // create a table that maps variable names to where the variable
        // "lives"
        let varmap = {
            let mut map = HashMap::new();
            for (i, name) in locals.clone().into_iter().enumerate() {
                map.insert(name.clone(), VariableLocation::Local(i));
            }
            let mut cellvars = freevars.clone();
            cellvars.extend(ocellvars.clone());
            for (i, name) in cellvars.into_iter().enumerate() {
                map.insert(name, VariableLocation::Cell(i));
            }
            map
        };

        // pass 3: generate opcodes
        let (code, lnotab) = {
            enum Task {
                FixLabel { offset: usize, label: Label },
            }
            struct State {
                code: Vec<usize>,
                lnotab: Vec<(usize, usize)>,
                tasks: Vec<Task>,
                labelmap: HashMap<Label, usize>,
            }
            impl State {
                fn pos(&self) -> usize {
                    self.code.len()
                }
                fn push(&mut self, value: usize) {
                    self.code.push(value);
                }
                fn push_label(&mut self, label: Label) {
                    let offset = self.pos();
                    self.push(std::usize::MAX);

                    // We might not know what the label value is, so
                    // save this as a task for later
                    self.tasks.push(Task::FixLabel { offset, label });
                }
                fn add(&mut self, opcode: usize, args: &[usize]) {
                    self.push(opcode);

                    // at a very basic level, check that:
                    //   - opcode is a real opcode, and
                    //   - the number of arguments match
                    let info = &opc::OPCODE_INFO_MAP[opcode];
                    assert_eq!(info.argtypes().len(), args.len());
                    for arg in args {
                        self.push(*arg);
                    }
                }
            }
            let mut state = State {
                code: Vec::new(),
                lnotab: Vec::new(),
                tasks: Vec::new(),
                labelmap: HashMap::new(),
            };
            for op in self.code {
                match op {
                    PseudoOpcode::LineNumber(i) => {
                        state.lnotab.push((state.pos(), i));
                    }
                    PseudoOpcode::Label(label) => {
                        state.labelmap.insert(label, state.pos());
                    }
                    PseudoOpcode::Pop => {
                        state.add(opc::POP_TOP, &[]);
                    }
                    PseudoOpcode::RotTwo => {
                        state.add(opc::ROT_TWO, &[]);
                    }
                    PseudoOpcode::SwapTos1Tos2 => {
                        state.add(opc::SWAP_TOS1_TOS2, &[]);
                    }
                    PseudoOpcode::PullTos2 => {
                        state.add(opc::PULL_TOS2, &[]);
                    }
                    PseudoOpcode::PullTos3 => {
                        state.add(opc::PULL_TOS3, &[]);
                    }
                    PseudoOpcode::DupTop => {
                        state.add(opc::DUP_TOP, &[]);
                    }
                    PseudoOpcode::LoadConst(i) => {
                        state.add(opc::LOAD_CONST, &[i]);
                    }
                    PseudoOpcode::LoadVar(name) => match varmap
                        .get(&self.symbol_registry.intern_rcstr(&name))
                        .unwrap()
                    {
                        VariableLocation::Local(i) => {
                            state.add(opc::LOAD_LOCAL, &[*i]);
                        }
                        VariableLocation::Cell(i) => {
                            state.add(opc::LOAD_DEREF, &[*i]);
                        }
                    },
                    PseudoOpcode::StoreVar(name) => match varmap
                        .get(&self.symbol_registry.intern_rcstr(&name))
                        .unwrap()
                    {
                        VariableLocation::Local(i) => {
                            state.add(opc::STORE_LOCAL, &[*i]);
                        }
                        VariableLocation::Cell(i) => {
                            state.add(opc::STORE_DEREF, &[*i]);
                        }
                    },
                    PseudoOpcode::LoadCell(name) => match varmap
                        .get(&self.symbol_registry.intern_rcstr(&name))
                        .unwrap()
                    {
                        VariableLocation::Cell(i) => {
                            state.add(opc::LOAD_CELL, &[*i]);
                        }
                        _ => panic!("LoadCell on non-cell var"),
                    },
                    PseudoOpcode::LoadDunderNew => {
                        state.add(opc::LOAD_DUNDER_NEW, &[]);
                    }
                    PseudoOpcode::LoadClassForNew => {
                        state.add(opc::LOAD_CLASS_FOR_NEW, &[]);
                    }
                    PseudoOpcode::Nonlocal(_) => {
                        // nonlocal is a no-op -- the primary
                        // purpose of nonlocal is to mark the given
                        // variable as being nonlocal, which is handled
                        // before the code-generation here
                    }
                    PseudoOpcode::Unpack(n) => {
                        state.add(opc::UNPACK_SEQUENCE, &[n]);
                    }
                    PseudoOpcode::MakeMutableString(namei) => {
                        state.add(opc::MAKE_MUTABLE_STRING, &[namei]);
                    }
                    PseudoOpcode::MakeList(argc) => {
                        state.add(opc::MAKE_LIST, &[argc]);
                    }
                    PseudoOpcode::MakeTable(argc) => {
                        state.add(opc::MAKE_TABLE, &[argc]);
                    }
                    PseudoOpcode::MakeMap(argc) => {
                        state.add(opc::MAKE_MAP, &[argc]);
                    }
                    PseudoOpcode::MakeMutableList(argc) => {
                        state.add(opc::MAKE_MUTABLE_LIST, &[argc]);
                    }
                    PseudoOpcode::MakeMutableMap(argc) => {
                        state.add(opc::MAKE_MUTABLE_MAP, &[argc]);
                    }
                    PseudoOpcode::LoadMethod(namei) => {
                        state.add(opc::LOAD_METHOD, &[namei]);
                    }
                    PseudoOpcode::CallFunction(lineno, argc) => {
                        state.add(opc::CALL_FUNCTION, &[lineno, argc]);
                    }
                    PseudoOpcode::CallFunctionGeneric(lineno) => {
                        state.add(opc::CALL_FUNCTION_GENERIC, &[lineno]);
                    }
                    PseudoOpcode::ExtendList(lineno) => {
                        state.add(opc::EXTEND_LIST, &[lineno]);
                    }
                    PseudoOpcode::ExtendTable(lineno) => {
                        state.add(opc::EXTEND_TABLE, &[lineno]);
                    }
                    PseudoOpcode::MakeFunction(i) => {
                        state.add(opc::MAKE_FUNCTION, &[i]);
                    }
                    PseudoOpcode::MakeClass(namei, is_trait) => {
                        state.add(opc::MAKE_CLASS, &[namei, is_trait]);
                    }
                    PseudoOpcode::MakeExceptionKind(namei) => {
                        state.add(opc::MAKE_EXCEPTION_KIND, &[namei]);
                    }
                    PseudoOpcode::Jump(label) => {
                        state.push(opc::JUMP);
                        state.push_label(label);
                    }
                    PseudoOpcode::PopJumpIfTrue(label) => {
                        state.push(opc::POP_JUMP_IF_TRUE);
                        state.push_label(label);
                    }
                    PseudoOpcode::PopJumpIfFalse(label) => {
                        state.push(opc::POP_JUMP_IF_FALSE);
                        state.push_label(label);
                    }
                    PseudoOpcode::JumpIfTrueOrPop(label) => {
                        state.push(opc::JUMP_IF_TRUE_OR_POP);
                        state.push_label(label);
                    }
                    PseudoOpcode::JumpIfFalseOrPop(label) => {
                        state.push(opc::JUMP_IF_FALSE_OR_POP);
                        state.push_label(label);
                    }
                    PseudoOpcode::GetIter => {
                        state.add(opc::GET_ITER, &[]);
                    }
                    PseudoOpcode::ForIter(label) => {
                        state.push(opc::FOR_ITER);
                        state.push_label(label);
                    }
                    PseudoOpcode::Binop(lineno, op) => match op {
                        Binop::Is => {
                            state.add(opc::BINARY_IS, &[]);
                        }
                        _ => {
                            state.add(
                                match op {
                                    Binop::Pow => opc::BINARY_POWER,
                                    Binop::Add => opc::BINARY_ADD,
                                    Binop::Sub => opc::BINARY_SUB,
                                    Binop::Mul => opc::BINARY_MUL,
                                    Binop::Div => opc::BINARY_DIV,
                                    Binop::TruncDiv => opc::BINARY_TRUNCDIV,
                                    Binop::Rem => opc::BINARY_REM,
                                    Binop::Lt => opc::BINARY_LT,
                                    Binop::Eq => opc::BINARY_EQ,
                                    Binop::Is => panic!("FUBAR"),
                                    _ => panic!("binop not yet supported {:?}", op),
                                },
                                &[lineno],
                            );
                        }
                    },
                    PseudoOpcode::Unop(lineno, op) => {
                        state.add(
                            match op {
                                Unop::Not => opc::UNARY_NOT,
                                Unop::Neg => opc::UNARY_NEG,
                                Unop::Pos => opc::UNARY_POS,
                            },
                            &[lineno],
                        );
                    }
                    PseudoOpcode::LoadAttribute(i) => {
                        state.add(opc::LOAD_ATTRIBUTE, &[i]);
                    }
                    PseudoOpcode::StoreAttribute(i) => {
                        state.add(opc::STORE_ATTRIBUTE, &[i]);
                    }
                    PseudoOpcode::LoadStaticAttribute(i) => {
                        state.add(opc::LOAD_STATIC_ATTRIBUTE, &[i]);
                    }
                    PseudoOpcode::Import(lineno, i) => {
                        state.add(opc::IMPORT, &[lineno, i]);
                    }
                    PseudoOpcode::Yield => {
                        state.add(opc::YIELD, &[]);
                    }
                    PseudoOpcode::Return => {
                        state.add(opc::RETURN, &[]);
                    }
                    PseudoOpcode::Breakpoint => {
                        state.add(opc::BREAKPOINT, &[]);
                    }
                }
            }
            for task in state.tasks {
                match task {
                    Task::FixLabel { offset, label } => {
                        state.code[offset] = state.labelmap[&label];
                    }
                }
            }
            (state.code, state.lnotab)
        };

        let argument_map = ArgumentMap::new(&self.parameter_info, &varmap);

        let short_name = short_name_from_full_name(&self.full_name);

        Ok(Code {
            kind: self.kind,
            code,
            names: self.symbol_registry.translate_vec(self.names),
            constants: self.constants,
            children: self.children,
            locals,
            freevars,
            ocellvars,
            parameter_info: self.parameter_info,
            argument_map,
            module_name: self.module_name,
            full_name: self.full_name,
            short_name: short_name,
            lineno: self.lineno,
            lnotab,
            doc: self.doc,
        })
    }
}
