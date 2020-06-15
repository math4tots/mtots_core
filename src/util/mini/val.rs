use super::FunctionDisplay;
use super::Scope;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, PartialEq)]
pub enum Val {
    Nil,
    Number(f64),

    /// Use of Rc<String> over Rc<str> is by design --
    /// this allows mini to interoperate with rest of mtots
    /// without copying the String all over the place
    String(Rc<String>),

    List(Rc<RefCell<Vec<Val>>>),

    Function(Rc<Function>),
}

impl std::fmt::Debug for Val {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Val::Nil => write!(f, "nil"),
            Val::Number(x) => write!(f, "{}", x),
            Val::String(s) => write!(f, "{:?}", s),
            Val::List(list) => write!(f, "{:?}", list.borrow()),
            Val::Function(x) => write!(f, "<function {}>", x.id()),
        }
    }
}

impl std::fmt::Display for Val {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Val::String(s) => write!(f, "{}", s),
            _ => write!(f, "{:?}", self),
        }
    }
}

impl Val {
    pub fn truthy(&self) -> bool {
        match self {
            Val::Nil => false,
            Val::Number(i) => *i != 0.0,
            Val::String(s) => !s.is_empty(),
            Val::List(list) => list.borrow().len() != 0,
            Val::Function(_) => true,
        }
    }
    pub fn apply(&self, args: Vec<Val>) -> Result<Val, String> {
        match self {
            Val::Function(f) => f.apply(args),
            _ => Err(format!("Only functions can be applied (got {:?})", self)),
        }
    }
    pub fn len(&self) -> Result<usize, String> {
        match self {
            Val::String(s) => Ok(s.len()),
            Val::List(list) => Ok(list.borrow().len()),
            _ => Err(format!("len requires string or list (got {:?})", self)),
        }
    }
    pub fn lenval(&self) -> Result<Val, String> {
        Ok(Val::Number(self.len()? as f64))
    }
    pub fn add(&self, other: &Val) -> Result<Val, String> {
        match (self, other) {
            (Val::Number(a), Val::Number(b)) => Ok(Val::Number(*a + *b)),
            (Val::String(a), Val::String(b)) => Ok(Val::String(Rc::new(format!("{}{}", a, b)))),
            (Val::List(a), Val::List(b)) => {
                let mut ret = Vec::new();
                ret.extend(a.borrow().clone());
                ret.extend(b.borrow().clone());
                Ok(Val::List(Rc::new(RefCell::new(ret))))
            }
            _ => Err(format!("add on invalid operands ({:?}, {:?})", self, other)),
        }
    }
    pub fn sub(&self, other: &Val) -> Result<Val, String> {
        match (self, other) {
            (Val::Number(a), Val::Number(b)) => Ok(Val::Number(*a - *b)),
            (Val::List(a), Val::List(b)) => {
                let mut ret = Vec::new();
                for x in a.borrow().iter() {
                    if !b.borrow().contains(x) {
                        ret.push(x.clone());
                    }
                }
                Ok(Val::List(Rc::new(RefCell::new(ret))))
            }
            _ => Err(format!("sub on invalid operands ({:?}, {:?})", self, other)),
        }
    }
    pub fn mul(&self, other: &Val) -> Result<Val, String> {
        match (self, other) {
            (Val::Number(a), Val::Number(b)) => Ok(Val::Number(*a * *b)),
            (Val::String(a), Val::Number(b)) => {
                let b = *b as usize;
                let mut ret = String::new();
                for _ in 0..b {
                    ret.push_str(a);
                }
                Ok(Val::String(Rc::new(ret)))
            }
            (Val::List(a), Val::Number(b)) => {
                let b = *b as usize;
                let mut ret = Vec::new();
                for _ in 0..b {
                    ret.extend(a.borrow().clone());
                }
                Ok(Val::List(Rc::new(RefCell::new(ret))))
            }
            _ => Err(format!("mul on invalid operands ({:?}, {:?})", self, other)),
        }
    }
    pub fn div(&self, other: &Val) -> Result<Val, String> {
        match (self, other) {
            (Val::Number(a), Val::Number(b)) => Ok(Val::Number(*a / *b)),
            _ => Err(format!("div on invalid operands ({:?}, {:?})", self, other)),
        }
    }
    pub fn truncdiv(&self, other: &Val) -> Result<Val, String> {
        match (self, other) {
            (Val::Number(a), Val::Number(b)) => Ok(Val::Number((*a / *b).trunc())),
            _ => Err(format!(
                "truncdiv on invalid operands ({:?}, {:?})",
                self, other
            )),
        }
    }
    pub fn rem(&self, other: &Val) -> Result<Val, String> {
        match (self, other) {
            (Val::Number(a), Val::Number(b)) => Ok(Val::Number(*a % *b)),
            _ => Err(format!("rem on invalid operands ({:?}, {:?})", self, other)),
        }
    }
    pub fn pos(&self) -> Result<Val, String> {
        match self {
            Val::Number(_) => Ok(self.clone()),
            _ => Err(format!("pos on invalid operand ({:?})", self)),
        }
    }
    pub fn neg(&self) -> Result<Val, String> {
        match self {
            Val::Number(x) => Ok(Val::Number(-*x)),
            _ => Err(format!("pos on invalid operand ({:?})", self)),
        }
    }
}

/// mini Function
/// Parameters follow javascript rules -- any extra arguments are ignored,
/// if not enough arguments are supplied, the missing ones are set to nil
pub struct Function {
    id: usize,
    scope: Rc<RefCell<Scope>>,
    display: Rc<FunctionDisplay>,
}

impl Function {
    pub fn new(scope: Rc<RefCell<Scope>>, display: Rc<FunctionDisplay>) -> Rc<Function> {
        let id = scope.borrow().new_id();
        Rc::new(Function { id, scope, display })
    }
    pub fn id(&self) -> usize {
        self.id
    }
    pub fn apply(&self, args: Vec<Val>) -> Result<Val, String> {
        let scope = Scope::new(self.scope.clone());
        let params = self.display.params();
        let mut args = args.into_iter();
        let mut done = false;
        for param in params {
            let next = if done {
                None
            } else {
                match args.next() {
                    Some(arg) => Some(arg),
                    None => {
                        done = true;
                        None
                    }
                }
            };
            match next {
                Some(arg) => scope.borrow_mut().set(param.clone(), arg),
                None => scope.borrow_mut().set(param.clone(), Val::Nil),
            }
        }
        self.display.body().eval(&scope)
    }
}

impl std::fmt::Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<function>")
    }
}

impl std::cmp::PartialEq for Function {
    fn eq(&self, other: &Function) -> bool {
        self.id == other.id
    }
}
