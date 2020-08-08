use crate::ArgSpec;
use crate::CallFunctionDesc;
use crate::CallMethodDesc;
use crate::Mark;
use crate::RcStr;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VariableType {
    Local,
    Upval,
}

#[derive(Clone)]
pub struct Variable {
    type_: VariableType,
    slot: usize,
    name: RcStr,
    mark: Mark,
}

impl fmt::Debug for Variable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Variable({:?}, {}, {})",
            self.type_, self.slot, self.name
        )
    }
}

impl Variable {
    pub fn type_(&self) -> VariableType {
        self.type_
    }
    pub fn slot(&self) -> usize {
        self.slot
    }
    pub fn name(&self) -> &RcStr {
        &self.name
    }
}

/// Description of the state of variables in a given scope
#[derive(Debug, Clone)]
pub struct VarSpec {
    local: Vec<(RcStr, Mark)>,
    free: Vec<(RcStr, Mark)>,
    owned: Vec<(RcStr, Mark)>,
    cache: RefCell<HashMap<RcStr, Option<Variable>>>,
}

impl VarSpec {
    pub fn new(
        local: Vec<(RcStr, Mark)>,
        free: Vec<(RcStr, Mark)>,
        owned: Vec<(RcStr, Mark)>,
    ) -> Self {
        let mut ret = Self {
            local,
            free,
            owned,
            cache: RefCell::new(HashMap::new()),
        };
        ret.sort();
        ret
    }
    fn sort(&mut self) {
        self.local.sort_by(|(a, _), (b, _)| a.cmp(b));
        self.free.sort_by(|(a, _), (b, _)| a.cmp(b));
        self.owned.sort_by(|(a, _), (b, _)| a.cmp(b));
    }
    pub fn local(&self) -> &Vec<(RcStr, Mark)> {
        &self.local
    }
    pub fn free(&self) -> &Vec<(RcStr, Mark)> {
        &self.free
    }
    pub fn owned(&self) -> &Vec<(RcStr, Mark)> {
        &self.owned
    }
    pub fn get(&self, name: &RcStr) -> Option<Variable> {
        if !self.cache.borrow().contains_key(name) {
            let result = self.get_uncached(name);
            self.cache.borrow_mut().insert(name.clone(), result);
        }
        self.cache.borrow().get(name).unwrap().clone()
    }
    fn get_uncached(&self, name: &RcStr) -> Option<Variable> {
        for (slot, (lname, mark)) in self.local.iter().enumerate() {
            if name == lname {
                return Some(Variable {
                    name: name.clone(),
                    type_: VariableType::Local,
                    slot,
                    mark: mark.clone(),
                });
            }
        }
        for (slot, (fname, mark)) in self.free.iter().chain(&self.owned).enumerate() {
            if name == fname {
                return Some(Variable {
                    name: name.clone(),
                    type_: VariableType::Upval,
                    slot,
                    mark: mark.clone(),
                });
            }
        }
        None
    }
}

pub struct ModuleDisplay {
    name: RcStr,
    body: Expr,
    varspec: Option<VarSpec>,
    doc: Option<RcStr>,
}

impl ModuleDisplay {
    pub(crate) fn new(name: RcStr, doc: Option<RcStr>, body: Expr) -> Self {
        Self {
            name,
            body,
            varspec: None,
            doc,
        }
    }
    pub fn name(&self) -> &RcStr {
        &self.name
    }
    pub fn body(&self) -> &Expr {
        &self.body
    }
    pub fn body_mut(&mut self) -> &mut Expr {
        &mut self.body
    }
    pub fn varspec(&self) -> &Option<VarSpec> {
        &self.varspec
    }
    pub fn varspec_mut(&mut self) -> &mut Option<VarSpec> {
        &mut self.varspec
    }
    pub fn doc(&self) -> &Option<RcStr> {
        &self.doc
    }
}

#[derive(Debug)]
pub struct Args {
    pub args: Vec<Expr>,
    pub varargs: Option<Box<Expr>>,
    pub kwargs: Vec<(RcStr, Expr)>,
    pub kwmap: Option<Box<Expr>>,
}

impl Args {
    pub fn new(
        args: Vec<Expr>,
        varargs: Option<Box<Expr>>,
        kwargs: Vec<(RcStr, Expr)>,
        kwmap: Option<Box<Expr>>,
    ) -> Self {
        Self {
            args,
            varargs,
            kwargs,
            kwmap,
        }
    }
    pub(crate) fn call_function_info(&self) -> CallFunctionDesc {
        CallFunctionDesc {
            argc: self.args.len(),
            variadic: self.varargs.is_some(),
            kwargs: self.kwargs.iter().map(|(name, _)| name.clone()).collect(),
            kwmap: self.kwmap.is_some(),
        }
    }
    pub(crate) fn call_method_info(&self, method_name: RcStr) -> CallMethodDesc {
        CallMethodDesc {
            argc: self.args.len(),
            kwargs: self.kwargs.iter().map(|(name, _)| name.clone()).collect(),
            method_name,
        }
    }
}

#[derive(Debug)]
pub struct Expr {
    mark: Mark,
    desc: ExprDesc,
}

impl Expr {
    pub fn new(mark: Mark, desc: ExprDesc) -> Self {
        Self { mark, desc }
    }
    pub fn mark(&self) -> &Mark {
        &self.mark
    }
    pub fn desc(&self) -> &ExprDesc {
        &self.desc
    }
    pub(crate) fn desc_mut(&mut self) -> &mut ExprDesc {
        &mut self.desc
    }
    pub fn unpack(self) -> (Mark, ExprDesc) {
        (self.mark, self.desc)
    }
}

#[derive(Debug)]
pub enum ExprDesc {
    Nil,
    Bool(bool),
    Number(f64),
    String(RcStr),
    Name(RcStr),
    List(Vec<Expr>),
    Map(Vec<(Expr, Expr)>),

    Parentheses(Box<Expr>),
    Block(Vec<Expr>),

    Switch(Box<Expr>, Vec<(Expr, Expr)>, Option<Box<Expr>>),
    If(Vec<(Expr, Expr)>, Option<Box<Expr>>),
    For(AssignTarget, Box<Expr>, Box<Expr>),
    While(Box<Expr>, Box<Expr>),

    Binop(Binop, Box<Expr>, Box<Expr>),
    LogicalBinop(LogicalBinop, Box<Expr>, Box<Expr>),
    Unop(Unop, Box<Expr>),
    Subscript(Box<Expr>, Box<Expr>),
    Slice(Box<Expr>, Option<Box<Expr>>, Option<Box<Expr>>),
    Attr(Box<Expr>, RcStr),
    CallFunction(Box<Expr>, Args),
    CallMethod(Box<Expr>, RcStr, Args),
    Assign(AssignTarget, Box<Expr>),
    AugAssign(AssignTarget, Binop, Box<Expr>),
    NonlocalAssign(RcStr, Box<Expr>),
    Nonlocal(Vec<RcStr>),

    New(Option<RcStr>, Vec<(RcStr, Expr)>),
    Del(RcStr),
    Yield(Box<Expr>),
    Return(Option<Box<Expr>>),

    Import(RcStr),
    BreakPoint,

    /// AssignDoc at runtime is more or less a nop.
    /// However, it attaches docstrings to field assignments
    /// (the first 'Expr' parameter should always be an Assign)
    AssignDoc(Box<Expr>, RcStr, RcStr),

    Function {
        is_generator: bool,
        name: Option<RcStr>,
        params: ArgSpec,
        docstr: Option<RcStr>,
        body: Box<Expr>,

        varspec: Option<VarSpec>,
    },
    Class {
        name: RcStr,
        bases: Vec<Expr>,
        docstr: Option<RcStr>,
        methods: Vec<(RcStr, Expr)>,
        static_methods: Vec<(RcStr, Expr)>,

        /// The hidden name for a class is computed during annotation.
        /// The hidden name is used by methods to refer to the class
        /// that they belong to
        hidden_name: Option<RcStr>,
    },
}

#[derive(Debug)]
pub struct AssignTarget {
    mark: Mark,
    desc: AssignTargetDesc,
}

impl AssignTarget {
    pub(crate) fn new(mark: Mark, desc: AssignTargetDesc) -> Self {
        Self { mark, desc }
    }
    pub fn unpack(self) -> (Mark, AssignTargetDesc) {
        (self.mark, self.desc)
    }
    pub fn mark(&self) -> &Mark {
        &self.mark
    }
    pub fn desc(&self) -> &AssignTargetDesc {
        &self.desc
    }
    pub(crate) fn desc_mut(&mut self) -> &mut AssignTargetDesc {
        &mut self.desc
    }
}

#[derive(Debug)]
pub enum AssignTargetDesc {
    Name(RcStr),
    List(Vec<AssignTarget>),
    Attr(Box<Expr>, RcStr),
    Subscript(Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone, Copy)]
pub enum Unop {
    Pos,
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy)]
pub enum Binop {
    Add,
    Sub,
    Mul,
    Div,
    TruncDiv,
    Rem,
    Pow,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
    Is,
    IsNot,
}

#[derive(Debug, Clone, Copy)]
pub enum LogicalBinop {
    And,
    Or,
}
