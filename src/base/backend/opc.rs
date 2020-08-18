use super::*;

#[derive(Debug)]
pub(crate) enum Opcode {
    Pop,
    Dup,
    Dup2,
    Pull2,
    Pull3,
    Unpull2,
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

    New(Box<Vec<RcStr>>),
    Del(Box<Variable>),
    Binop(Binop),
    Unop(Unop),
    GetItem,
    SetItem,
    TeeItem,

    Iter,
    Next,

    Import(RcStr),
    Yield,
    Await,
    Return,
    Jump(usize),
    JumpIfFalse(usize),
    TeeJumpIfFalse(usize),
    TeeJumpIfTrue(usize),
    CallFunction(Box<CallFunctionDesc>),
    CallMethod(Box<CallMethodDesc>),

    NewFunction(Box<NewFunctionDesc>),
    NewClass(Box<NewClassDesc>),
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
    pub variadic: bool,
    pub kwargs: Vec<RcStr>,
    pub kwmap: bool,
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

    pub kind: FunctionKind,
}

#[derive(Debug)]
pub(crate) struct NewClassDesc {
    pub name: RcStr,
    pub nbases: usize,
    pub method_names: Vec<RcStr>,
    pub static_method_names: Vec<RcStr>,
}

#[inline(always)]
pub(super) fn step(globals: &mut Globals, code: &Code, frame: &mut Frame) -> StepResult {
    let (pc, opc) = frame.fetch(code);
    let opc = if let Some(opc) = opc {
        opc
    } else {
        assert_eq!(
            frame.len(),
            1,
            "Bad stack size: expected = 1, actual = {}, pc = {}, opslen = {}, codename = {}",
            frame.len(),
            pc,
            code.ops().len(),
            code.name(),
        );
        return StepResult::Return(frame.pop());
    };

    macro_rules! addtrace {
        () => {
            globals.trace_push(code.marks()[pc].clone());
        };
    }

    macro_rules! get0 {
        ($expr:expr) => {{
            // We explicitly borrow a reference to _g here so that
            // it can't be used inside $expr.
            // If globals is required, we really need to use `get1!`
            let _g = &globals;
            match $expr {
                Ok(t) => t,
                Err(error) => {
                    #[allow(path_statements)]
                    {
                        _g;
                    }
                    addtrace!();
                    return StepResult::Err(error);
                }
            }
        }};
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
        Opcode::Dup2 => {
            frame.pushn(frame.peekn(2));
        }
        Opcode::Pull2 => {
            frame.pull(2);
        }
        Opcode::Pull3 => {
            frame.pull(3);
        }
        Opcode::Unpull2 => {
            frame.unpull(2);
        }
        Opcode::Swap01 => {
            frame.swap(0, 1);
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
            let map = {
                let mut iter = frame.popn_iter(2 * len);
                let mut map = IndexMap::new();
                while let Some(key) = iter.next() {
                    let key = get0!(Key::try_from(key));
                    let value = iter.next().unwrap();
                    map.insert(key, value);
                }
                map
            };
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
            let r = owner.getattr(globals, attr);
            let value = get0!(r);
            frame.push(value);
        }
        Opcode::SetAttr(attr) => {
            let owner = frame.pop();
            let value = frame.pop();
            let r = owner.setattr(globals, attr, value);
            get0!(r);
        }
        Opcode::TeeAttr(attr) => {
            let owner = frame.pop();
            let value = frame.peek().clone();
            let r = owner.setattr(globals, attr, value);
            get0!(r);
        }
        Opcode::New(argnames) => {
            let argvals = frame.popn_iter(argnames.len()).map(RefCell::new);
            let map = argnames
                .iter()
                .map(Clone::clone)
                .zip(argvals)
                .collect::<HashMap<_, _>>();
            let cls = get0!(frame.pop().into_class());
            let table = Table::new(cls, map);
            frame.push(table.into());
        }
        Opcode::Del(var) => {
            let value = get0!(frame.delvar(var));
            frame.push(value);
        }
        Opcode::Binop(op) => {
            let rhs = frame.pop();
            let lhs = frame.pop();

            // for arithmetic operations it's critical to keep overhead
            // as low as possible, but in all other cases we call a method
            // corresponding to each operation instead
            let result = match op {
                Binop::Add => match lhs {
                    Value::Number(a) => Value::Number(a + get0!(rhs.number())),
                    _ => get1!(lhs.apply_method(globals, "__add", vec![rhs], None)),
                },
                Binop::Sub => match lhs {
                    Value::Number(a) => Value::Number(a - get0!(rhs.number())),
                    _ => get1!(lhs.apply_method(globals, "__sub", vec![rhs], None)),
                },
                Binop::Mul => match lhs {
                    Value::Number(a) => Value::Number(a * get0!(rhs.number())),
                    _ => get1!(lhs.apply_method(globals, "__mul", vec![rhs], None)),
                },
                Binop::Div => match lhs {
                    Value::Number(a) => Value::Number(a / get0!(rhs.number())),
                    _ => get1!(lhs.apply_method(globals, "__div", vec![rhs], None)),
                },
                Binop::TruncDiv => match lhs {
                    Value::Number(a) => Value::Number((a / get0!(rhs.number())).trunc()),
                    _ => get1!(lhs.apply_method(globals, "__truncdiv", vec![rhs], None)),
                },
                Binop::Rem => match lhs {
                    Value::Number(a) => Value::Number(a % get0!(rhs.number())),
                    _ => get1!(lhs.apply_method(globals, "__rem", vec![rhs], None)),
                },
                Binop::Pow => match lhs {
                    Value::Number(a) => Value::Number(a.powf(get0!(rhs.number()))),
                    _ => get1!(lhs.apply_method(globals, "__pow", vec![rhs], None)),
                },
                Binop::Lt => Value::from(get0!(lhs.lt(&rhs))),
                Binop::Le => Value::from(!get0!(rhs.lt(&lhs))),
                Binop::Gt => Value::from(get0!(rhs.lt(&lhs))),
                Binop::Ge => Value::from(!get0!(lhs.lt(&rhs))),
                Binop::Eq => Value::from(lhs == rhs),
                Binop::Ne => Value::from(lhs != rhs),
                Binop::Is => Value::from(lhs.is(&rhs)),
                Binop::IsNot => Value::from(!lhs.is(&rhs)),
            };
            frame.push(result);
        }
        Opcode::Unop(op) => {
            let arg = frame.pop();

            macro_rules! operr {
                () => {
                    err!(
                        "Unop {:?} not supported for {:?}",
                        op,
                        arg.get_class(globals)
                    )
                };
            }

            let result = match op {
                Unop::Pos => match arg {
                    Value::Number(_) => arg,
                    _ => operr!(),
                },
                Unop::Neg => match arg {
                    Value::Number(a) => Value::Number(-a),
                    _ => operr!(),
                },
                Unop::Not => Value::from(!arg.truthy()),
            };
            frame.push(result);
        }
        Opcode::GetItem => {
            let index = frame.pop();
            let owner = frame.pop();
            let value = get1!(owner.getitem(globals, index));
            frame.push(value);
        }
        Opcode::SetItem => {
            let index = frame.pop();
            let owner = frame.pop();
            let value = frame.pop();
            get1!(owner.setitem(globals, index, value));
        }
        Opcode::TeeItem => {
            let index = frame.pop();
            let owner = frame.pop();
            let value = frame.peek();
            get1!(owner.setitem(globals, index, value.clone()));
        }
        Opcode::Iter => {
            let container = frame.pop();
            let iter = get1!(container.iter(globals));
            frame.push(iter);
        }
        Opcode::Next => {
            // peeks at the top of stack (should be an iterator)
            // and pushes two values: the value, and true/false depending
            // on whether there was a next value
            let iter = frame.peek();
            addtrace!();
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
                    return StepResult::Err(error);
                }
            }
            globals.trace_pop();
        }
        Opcode::Yield => {
            let value = frame.pop();
            return StepResult::Yield(value);
        }
        Opcode::Await => {
            let value = frame.pop();
            let promise = get0!(value.into_promise());
            return StepResult::Await(promise);
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
            let kwmap = if desc.kwmap {
                let map = get0!(frame.pop().into_map());
                Some(get0!(map.to_string_keys()))
            } else {
                None
            };
            let kwargs = if desc.kwargs.is_empty() {
                kwmap
            } else {
                let mut map = kwmap.unwrap_or_else(HashMap::new);
                let values = frame.popn(desc.kwargs.len());
                for (key, val) in desc.kwargs.iter().map(Clone::clone).zip(values) {
                    match map.entry(key) {
                        Entry::Occupied(_) => {}
                        Entry::Vacant(entry) => {
                            entry.insert(val);
                        }
                    }
                }
                Some(map)
            };
            let args = if desc.variadic {
                let extra_args = get1!(frame.pop().unpack(globals));
                let mut vec = frame.popn(desc.argc);
                vec.extend(extra_args);
                vec
            } else {
                frame.popn(desc.argc)
            };
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
            let func = Function::new(desc.argspec.clone(), desc.code.clone(), bindings, desc.kind);
            frame.push(func.into());
        }
        Opcode::NewClass(desc) => {
            let static_methods = frame.popn_iter(desc.static_method_names.len());
            let static_map = desc
                .static_method_names
                .iter()
                .map(Clone::clone)
                .zip(static_methods)
                .collect::<HashMap<_, _>>();

            let methods = frame.popn(desc.method_names.len());
            let map = desc
                .method_names
                .iter()
                .map(Clone::clone)
                .zip(methods)
                .collect::<HashMap<_, _>>();
            let bases: Vec<_> = get0!(frame
                .popn_iter(desc.nbases)
                .map(Value::into_class)
                .collect());
            let map = Class::join_class_maps(map, bases);

            let cls = Class::new(desc.name.clone(), map, static_map);

            frame.push(cls.into());
        }
    }
    StepResult::Ok
}

pub(super) enum StepResult {
    Ok,
    Return(Value),
    Yield(Value),
    Await(Rc<RefCell<Promise>>),
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
