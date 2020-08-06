use super::*;

#[derive(Debug)]
pub(crate) enum Opcode {
    Pop,
    Dup,
    Swap01,
    Unpack(u32),

    Nil,
    Bool(bool),
    Number(f64),
    String(RcStr),

    NewList(u32),
    NewMap(u32),

    GetVar(Box<Variable>),
    SetVar(Box<Variable>),
    TeeVar(Box<Variable>),
    GetAttr(RcStr),
    SetAttr(RcStr),
    TeeAttr(RcStr),

    Binop(Binop),

    Iter,
    Next,

    Import(RcStr),
    Yield,
    Return,
    Jump(usize),
    JumpIfFalse(usize),
    TeeJumpIfFalse(usize),
    TeeJumpIfTrue(usize),
    CallFunction(Box<CallFunctionDesc>),
    CallMethod(Box<CallMethodDesc>),

    NewFunction(Box<NewFunctionDesc>),
}

impl Opcode {
    pub(crate) fn patch_jump(&mut self, dest: usize) {
        match self {
            Self::Jump(d) => *d = dest,
            Self::JumpIfFalse(d) => *d = dest,
            Self::TeeJumpIfFalse(d) => *d = dest,
            Self::TeeJumpIfTrue(d) => *d = dest,
            _ => panic!("patch_jump on non-jump: {:?}", self),
        }
    }
}

#[derive(Debug)]
pub(crate) struct CallFunctionDesc {
    pub argc: usize,
    pub kwargs: Vec<RcStr>,
}

#[derive(Debug)]
pub(crate) struct CallMethodDesc {
    pub argc: usize,
    pub kwargs: Vec<RcStr>,
    pub method_name: RcStr,
}

#[derive(Debug)]
pub(crate) struct NewFunctionDesc {
    pub code: Rc<Code>,
    pub argspec: Rc<ArgSpec>,

    /// The list of upval slots that are needed to create the bindings
    /// for this new function
    pub freevar_binding_slots: Vec<usize>,

    pub is_generator: bool,
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
        Opcode::Swap01 => {
            frame.swap01();
        }
        Opcode::Unpack(len) => {
            let packed = frame.pop();
            let vec = get1!(packed.unpack(globals));
            if vec.len() != *len as usize {
                err!("Expected {} elements but got {}", len, vec.len())
            }
            frame.pushn(vec);
        }
        Opcode::Nil => frame.push(Value::Nil),
        Opcode::Bool(b) => frame.push(Value::Bool(*b)),
        Opcode::Number(x) => frame.push(Value::Number(*x)),
        Opcode::String(x) => frame.push(Value::String(x.clone())),
        Opcode::NewList(len) => {
            let len = *len as usize;
            let vec = frame.popn(len);
            frame.push(vec.into());
        }
        Opcode::NewMap(len) => {
            let len = *len as usize;
            let mut iter = frame.popn(2 * len).into_iter();
            let mut map = IndexMap::new();
            while let Some(key) = iter.next() {
                let key = get0!(Key::try_from(key));
                let value = iter.next().unwrap();
                map.insert(key, value);
            }
            frame.push(map.into());
        }
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

            macro_rules! operr {
                () => {
                    err!(
                        "Binop {:?} not supported for {:?}",
                        op,
                        lhs.get_class(globals)
                    )
                };
            }

            let result = match op {
                Binop::Add => match lhs {
                    Value::Number(a) => Value::Number(a + get0!(rhs.number())),
                    Value::String(a) => Value::String({
                        let mut string = a.unwrap_or_clone();
                        string.push_str(get0!(rhs.string()));
                        string.into()
                    }),
                    _ => operr!(),
                },
                Binop::Lt => match lhs {
                    Value::Number(a) => Value::Bool(a < get0!(rhs.number())),
                    _ => operr!(),
                },
                _ => panic!("TODO step Binop {:?}", op),
            };
            frame.push(result);
        }
        Opcode::Iter => {
            let container = frame.pop();
            let iter = get0!(container.iter(globals));
            frame.push(iter);
        }
        Opcode::Next => {
            // peeks at the top of stack (should be an iterator)
            // and pushes two values: the value, and true/false depending
            // on whether there was a next value
            let iter = frame.peek();
            match iter.resume(globals, Value::Nil) {
                ResumeResult::Yield(value) => {
                    frame.push(value);
                    frame.push(true.into());
                }
                ResumeResult::Return(value) => {
                    frame.push(value);
                    frame.push(false.into());
                }
                ResumeResult::Err(error) => {
                    addtrace!();
                    return StepResult::Err(error);
                }
            }
        }
        Opcode::Yield => {
            let value = frame.pop();
            return StepResult::Yield(value);
        }
        Opcode::Return => {
            let value = frame.pop();
            return StepResult::Return(value);
        }
        Opcode::Import(path) => {
            let module = get1!(globals.load(path).map(Clone::clone));
            frame.push(module.into());
        }
        Opcode::Jump(dest) => {
            frame.jump(*dest);
        }
        Opcode::JumpIfFalse(dest) => {
            if !frame.pop().truthy() {
                frame.jump(*dest);
            }
        }
        Opcode::TeeJumpIfFalse(dest) => {
            if frame.peek().truthy() {
                frame.pop();
            } else {
                frame.jump(*dest);
            }
        }
        Opcode::TeeJumpIfTrue(dest) => {
            if frame.peek().truthy() {
                frame.jump(*dest);
            } else {
                frame.pop();
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
            let func = frame.pop();
            let result = get1!(func.apply(globals, args, kwargs));
            frame.push(result);
        }
        Opcode::CallMethod(desc) => {
            let kwargs = if desc.kwargs.is_empty() {
                None
            } else {
                let values = frame.popn(desc.kwargs.len());
                Some(desc.kwargs.iter().map(Clone::clone).zip(values).collect())
            };
            let args = frame.popn(desc.argc);
            let owner = frame.pop();
            let result = get1!(owner.apply_method(globals, &desc.method_name, args, kwargs));
            frame.push(result);
        }
        Opcode::NewFunction(desc) => {
            let bindings: Vec<_> = desc
                .freevar_binding_slots
                .iter()
                .map(|slot| frame.getcell(*slot))
                .collect();
            let func = Function::new(
                desc.argspec.clone(),
                desc.code.clone(),
                bindings,
                desc.is_generator,
            );
            frame.push(func.into());
        }
    }
    StepResult::Ok
}

pub(super) enum StepResult {
    Ok,
    Return(Value),
    Yield(Value),
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
