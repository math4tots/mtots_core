use super::VarScope;
use std::rc::Rc;

pub enum Opcode {
    // Load constants
    Nil,
    Bool(bool),
    Number(f64),
    String(Rc<String>),
    NewList,
    NewFunc(Rc<Code>),

    // variable access
    Get(VarScope, u32),
    Set(VarScope, u32),
    Tee(VarScope, u32),

    // operators
    Return,
    CallFunc(u32),
    Print,
    Binop(Binop),
}

#[derive(Debug)]
pub enum Binop {
    Add,
    Subtract,
    Multiply,
    Divide,
    TruncDivide,
    Remainder,
}

pub struct Code {
    pub name: Rc<String>,
    pub nparams: usize,
    pub locals: Vec<Rc<String>>,
    pub ops: Vec<Opcode>,
}
