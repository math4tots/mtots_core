use super::Val;
use std::rc::Rc;

#[derive(Debug)]
pub enum Node {
    Constant(Val),
    GetVar(Rc<String>),
    SetVar(Rc<String>, Box<Node>),
    Block(Vec<Node>),
    ListDisplay(Vec<Node>),
    FunctionDisplay(Rc<FunctionDisplay>),
    If(Box<Node>, Box<Node>, Box<Node>),
    While(Box<Node>, Box<Node>),
    Operation(Operator, Vec<Node>),
}

#[derive(Debug)]
pub struct FunctionDisplay {
    params: Vec<Rc<String>>,
    body: Node,
}

impl FunctionDisplay {
    pub fn new(params: Vec<Rc<String>>, body: Node) -> FunctionDisplay {
        FunctionDisplay {
            params,
            body,
        }
    }
    pub fn params(&self) -> &Vec<Rc<String>> {
        &self.params
    }
    pub fn body(&self) -> &Node {
        &self.body
    }
}

macro_rules! define_operators {
    (
        $( $name:ident ),* $(,)?
    ) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Operator {
            $( $name ),*
        }
        impl Operator {
            pub fn list() -> &'static [Operator] {
                &[
                    $( Operator::$name ),*
                ]
            }
            pub fn from_str(s: &str) -> Option<Operator> {
                match s {
                    $( stringify!($name) => Some(Operator::$name) , )*
                    _ => None,
                }
            }
        }
    };
}

define_operators! {
    Apply, // i.e. call a function
    Len,   // len of a string or list
    Add,   // add strings, numbers or list together
    Sub,
    Mul,
    Div,
    TruncDiv,
    Rem,
    Pos,
    Neg,
    Print,
}
