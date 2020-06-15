use super::Function;
use super::Node;
use super::Operator;
use super::Scope;
use super::Val;
use std::cell::RefCell;
use std::rc::Rc;

impl Node {
    pub fn eval(&self, scope: &Rc<RefCell<Scope>>) -> Result<Val, String> {
        match self {
            Node::Constant(v) => Ok(v.clone()),
            Node::GetVar(name) => match scope.borrow().get(name) {
                Some(v) => Ok(v),
                None => Err(format!("Variable {:?} not found", name)),
            },
            Node::SetVar(name, expr) => {
                let val = expr.eval(scope)?;
                scope.borrow_mut().set(name.clone(), val);
                Ok(Val::Nil)
            }
            Node::Block(exprs) => {
                let mut last = Val::Nil;
                for expr in exprs {
                    last = expr.eval(scope)?;
                }
                Ok(last)
            }
            Node::ListDisplay(exprs) => {
                let result: Result<Vec<_>, _> = exprs.iter().map(|expr| expr.eval(scope)).collect();
                let vals = result?;
                Ok(Val::List(Rc::new(RefCell::new(vals))))
            }
            Node::FunctionDisplay(display) => {
                Ok(Val::Function(Function::new(scope.clone(), display.clone())))
            }
            Node::If(cond, body, other) => {
                let node = if cond.eval(scope)?.truthy() {
                    body
                } else {
                    other
                };
                node.eval(scope)
            }
            Node::While(cond, body) => {
                while cond.eval(scope)?.truthy() {
                    body.eval(scope)?;
                }
                Ok(Val::Nil)
            }
            Node::Operation(op, args) => {
                let argsr: Result<Vec<_>, _> = args.iter().map(|expr| expr.eval(scope)).collect();
                let mut args = Args::new(argsr?.into_iter());
                match op {
                    Operator::Apply => {
                        let f = args.next();
                        f.apply(args.rem())
                    }
                    Operator::Len => args.next().lenval(),
                    Operator::Add => {
                        let lhs = args.next();
                        let rhs = args.next();
                        lhs.add(&rhs)
                    }
                    Operator::Sub => {
                        let lhs = args.next();
                        let rhs = args.next();
                        lhs.sub(&rhs)
                    }
                    Operator::Mul => {
                        let lhs = args.next();
                        let rhs = args.next();
                        lhs.mul(&rhs)
                    }
                    Operator::Div => {
                        let lhs = args.next();
                        let rhs = args.next();
                        lhs.div(&rhs)
                    }
                    Operator::TruncDiv => {
                        let lhs = args.next();
                        let rhs = args.next();
                        lhs.truncdiv(&rhs)
                    }
                    Operator::Rem => {
                        let lhs = args.next();
                        let rhs = args.next();
                        lhs.rem(&rhs)
                    }
                    Operator::Pos => args.next().pos(),
                    Operator::Neg => args.next().neg(),
                    Operator::Print => {
                        let val = args.next();
                        (scope.borrow().opts().print)(&val)
                    }
                }
            }
        }
    }
}

struct Args {
    iter: std::vec::IntoIter<Val>,
    done: bool,
}

impl Args {
    fn new(iter: std::vec::IntoIter<Val>) -> Args {
        Args { iter, done: false }
    }
    fn next(&mut self) -> Val {
        if self.done {
            Val::Nil
        } else {
            match self.iter.next() {
                Some(val) => val,
                None => Val::Nil,
            }
        }
    }
    fn rem(self) -> Vec<Val> {
        if self.done {
            vec![]
        } else {
            self.iter.collect()
        }
    }
}
