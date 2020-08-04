use super::*;

#[derive(Debug)]
pub enum Opcode {
    Pop,
    Dup,
    Unpack(u32),

    Nil,
    Bool(bool),
    Number(f64),
    String(RcStr),

    GetVar(Box<Variable>),
    SetVar(Box<Variable>),
    TeeVar(Box<Variable>),
    GetAttr(RcStr),
    SetAttr(RcStr),
    TeeAttr(RcStr),

    Binop(Binop),

    Return,
    Jump(usize),
    JumpIfFalse(usize),
    CallFunction(Box<CallFunctionDesc>),

    NewFunction(Box<NewFunctionDesc>),
}

impl Opcode {
    pub(crate) fn patch_jump(&mut self, dest: usize) {
        match self {
            Self::Jump(d) => *d = dest,
            Self::JumpIfFalse(d) => *d = dest,
            _ => panic!("patch_jump on non-jump: {:?}", self),
        }
    }
}

#[derive(Debug)]
pub struct CallFunctionDesc {
    pub argc: usize,
    pub kwargs: Vec<RcStr>,
    pub method: Option<RcStr>,
}

#[derive(Debug)]
pub struct NewFunctionDesc {
    pub code: Rc<Code>,
    pub argspec: Rc<ArgSpec>,

    /// The list of upval slots that are needed to create the bindings
    /// for this new function
    pub freevar_binding_slots: Vec<usize>,
}

#[inline(always)]
pub(super) fn step(globals: &mut Globals, code: &Code, frame: &mut Frame) -> StepResult {
    let (pc, opc) = frame.fetch(code);
    let opc = if let Some(opc) = opc {
        opc
    } else {
        assert_eq!(frame.len(), 1);
        return StepResult::Return(frame.pop());
    };

    macro_rules! addtrace {
        () => {
            globals.trace_push(code.marks()[pc].clone());
        };
    }

    macro_rules! get0 {
        ($expr:expr) => {
            match $expr {
                Ok(t) => t,
                Err(error) => {
                    addtrace!();
                    return StepResult::Err(error);
                }
            }
        };
    }

    macro_rules! get1 {
        ($expr:expr) => {{
            addtrace!();
            match $expr {
                Ok(t) => {
                    globals.trace_pop();
                    t
                }
                Err(error) => {
                    return StepResult::Err(error);
                }
            }
        };};
    }

    macro_rules! err {
        ( $($arg:tt)+ ) => {
            {
                addtrace!();
                return StepResult::Err(rterr!($($arg)+));
            }
        };
    }

    match opc {
        Opcode::Pop => {
            frame.pop();
        }
        Opcode::Dup => {
            frame.push(frame.peek().clone());
        }
        Opcode::Unpack(len) => {
            let packed = frame.pop();
            let vec = get1!(packed.unpack(globals));
            if vec.len() != *len as usize {
                err!("Expected {} elements but got {}", len, vec.len())
            }
        }
        Opcode::Nil => frame.push(Value::Nil),
        Opcode::Bool(b) => frame.push(Value::Bool(*b)),
        Opcode::Number(x) => frame.push(Value::Number(*x)),
        Opcode::String(x) => frame.push(Value::String(x.clone())),
        Opcode::GetVar(var) => {
            let value = get0!(frame.getvar(var));
            frame.push(value);
        }
        Opcode::SetVar(var) => {
            let value = frame.pop();
            frame.setvar(var, value);
        }
        Opcode::TeeVar(var) => {
            let value = frame.peek().clone();
            frame.setvar(var, value);
        }
        Opcode::GetAttr(attr) => {
            let owner = frame.pop();
            let value = get0!(owner.getattr(attr));
            frame.push(value);
        }
        Opcode::SetAttr(attr) => {
            let owner = frame.pop();
            let value = frame.pop();
            get0!(owner.setattr(attr, value));
        }
        Opcode::TeeAttr(attr) => {
            let owner = frame.pop();
            let value = frame.peek().clone();
            get0!(owner.setattr(attr, value));
        }
        Opcode::Binop(op) => {
            let rhs = frame.pop();
            let lhs = frame.pop();
            let result = match op {
                Binop::Add => {
                    match lhs {
                        Value::Number(a) => Value::Number(a + get0!(rhs.number())),
                        _ => err!("Binop {:?} not supported for {:?}", op, lhs.get_class(globals)),
                    }
                }
                _ => panic!("TODO step Binop {:?}", op),
            };
            frame.push(result);
        }
        Opcode::Return => {
            let value = frame.pop();
            return StepResult::Return(value);
        }
        Opcode::Jump(dest) => {
            frame.jump(*dest);
        }
        Opcode::JumpIfFalse(dest) => {
            if !frame.pop().truthy() {
                frame.jump(*dest);
            }
        }
        Opcode::CallFunction(desc) => {
            let kwargs = if desc.kwargs.is_empty() {
                None
            } else {
                let values = frame.popn(desc.kwargs.len());
                Some(desc.kwargs.iter().map(Clone::clone).zip(values).collect())
            };
            let args = frame.popn(desc.argc);
            if let Some(method) = &desc.method {
                let owner = frame.pop();
                let result = get1!(owner.apply_method(globals, method, args, kwargs));
                frame.push(result);
            } else {
                let func = frame.pop();
                let result = get1!(func.apply(globals, args, kwargs));
                frame.push(result);
            }
        }
        Opcode::NewFunction(desc) => {
            let bindings: Vec<_> = desc
                .freevar_binding_slots
                .iter()
                .map(|slot| frame.getcell(*slot))
                .collect();
            let func = Function::new(desc.argspec.clone(), desc.code.clone(), bindings);
            frame.push(func.into());
        }
    }
    StepResult::Ok
}

pub(super) enum StepResult {
    Ok,
    Return(Value),
    Err(Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opcode_size() {
        assert_eq!(
            std::mem::size_of::<Opcode>(),
            std::mem::size_of::<usize>() + std::mem::size_of::<f64>()
        );
    }
}
