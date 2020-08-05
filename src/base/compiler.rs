use crate::base::ast::*;
use crate::Code;
use crate::Error;
use crate::Mark;
use crate::NewFunctionDesc;
use crate::Opcode;
use crate::RcStr;
use crate::Result;

const INVALID_JUMP: usize = usize::MAX;

pub fn compile(md: &ModuleDisplay) -> Result<Code> {
    let mut builder = Builder::new(
        Type::Module,
        md.name().clone(),
        md.varspec().clone().unwrap(),
    );
    builder.expr(md.body(), true)?;
    Ok(builder.build())
}

enum Type {
    Module,
    Function,
}

struct Builder {
    type_: Type,
    name: RcStr,
    varspec: VarSpec,
    ops: Vec<Opcode>,
    marks: Vec<Mark>,
    params: Vec<Variable>,
}

impl Builder {
    fn new(type_: Type, name: RcStr, varspec: VarSpec) -> Self {
        Self {
            type_,
            name,
            varspec,
            ops: vec![],
            marks: vec![],
            params: vec![],
        }
    }
    fn build(self) -> Code {
        assert_eq!(self.ops.len(), self.marks.len());
        Code::new(self.name, self.ops, self.params, self.varspec, self.marks)
    }
    fn add(&mut self, op: Opcode, mark: Mark) -> usize {
        let id = self.ops.len();
        self.ops.push(op);
        self.marks.push(mark);
        id
    }
    fn len(&self) -> usize {
        self.ops.len()
    }
}

impl Builder {
    fn expr(&mut self, expr: &Expr, used: bool) -> Result<()> {
        let mark = expr.mark().clone();
        match expr.desc() {
            ExprDesc::Nil => {
                if used {
                    self.add(Opcode::Nil, mark);
                }
            }
            ExprDesc::Bool(x) => {
                if used {
                    self.add(Opcode::Bool(*x), mark);
                }
            }
            ExprDesc::Number(x) => {
                if used {
                    self.add(Opcode::Number(*x), mark);
                }
            }
            ExprDesc::String(x) => {
                if used {
                    self.add(Opcode::String(x.clone()), mark);
                }
            }
            ExprDesc::Name(name) => {
                let variable = self.varspec.get(name).unwrap();
                self.add(Opcode::GetVar(variable.into()), mark.clone());
                if !used {
                    self.add(Opcode::Pop, mark);
                }
            }
            ExprDesc::Parentheses(expr) => {
                self.expr(expr, used)?;
            }
            ExprDesc::Block(exprs) => {
                if used {
                    match exprs.split_last() {
                        None => {
                            self.add(Opcode::Nil, mark.clone());
                        }
                        Some((last, subexprs)) => {
                            for subexpr in subexprs {
                                self.expr(subexpr, false)?;
                            }
                            self.expr(last, true)?;
                        }
                    }
                } else {
                    for expr in exprs {
                        self.expr(expr, false)?;
                    }
                }
            }
            ExprDesc::If(pairs, other) => {
                let mut end_jumps = Vec::new();
                let mut last = None;
                for (cond, body) in pairs {
                    if let Some(last) = last {
                        self.patch_jump(last);
                    }
                    self.expr(cond, true)?;
                    last = Some(self.add(Opcode::JumpIfFalse(INVALID_JUMP), mark.clone()));
                    self.expr(body, used)?;
                    end_jumps.push(self.add(Opcode::Jump(INVALID_JUMP), mark.clone()));
                }
                if let Some(last) = last {
                    self.patch_jump(last);
                }
                if let Some(other) = other {
                    self.expr(other, used)?;
                } else if used {
                    self.add(Opcode::Nil, mark);
                }
                for id in end_jumps {
                    self.patch_jump(id);
                }
            }
            ExprDesc::While(cond, body) => {
                let start_label = self.len();
                self.expr(cond, true)?;
                let cond_jump_id = self.add(Opcode::JumpIfFalse(INVALID_JUMP), mark.clone());
                self.expr(body, false)?;
                self.add(Opcode::Jump(start_label), mark.clone());
                self.patch_jump(cond_jump_id);
                if used {
                    self.add(Opcode::Nil, mark);
                }
            }
            ExprDesc::Binop(op, lhs, rhs) => {
                self.expr(lhs, true)?;
                self.expr(rhs, true)?;
                self.add(Opcode::Binop(*op), mark);
            }
            ExprDesc::Attr(owner, attr) => {
                self.expr(owner, true)?;
                self.add(Opcode::GetAttr(attr.clone()), mark.clone());
                if !used {
                    self.add(Opcode::Pop, mark);
                }
            }
            ExprDesc::CallFunction(f, args) => {
                self.expr(f, true)?;
                self.args(args)?;
                let info = args.call_function_info();
                self.add(Opcode::CallFunction(info.into()), mark.clone());
                if !used {
                    self.add(Opcode::Pop, mark);
                }
            }
            ExprDesc::Assign(target, valexpr) => {
                self.expr(valexpr, true)?;
                self.target(target, !used)?;
            }
            ExprDesc::NonlocalAssign(name, valexpr) => {
                self.expr(valexpr, true)?;
                let variable = self.varspec.get(name).unwrap();
                if used {
                    self.add(Opcode::TeeVar(variable.into()), mark);
                } else {
                    self.add(Opcode::SetVar(variable.into()), mark);
                }
            }
            ExprDesc::Return(valexpr) => {
                match self.type_ {
                    Type::Module => {
                        return Err(Error::rt(format!("Return is not allowed here").into(), vec![mark]));
                    }
                    Type::Function => {}
                }
                if let Some(valexpr) = valexpr {
                    self.expr(valexpr, true)?;
                } else {
                    self.add(Opcode::Nil, mark.clone());
                }
                self.add(Opcode::Return, mark);
            }
            ExprDesc::Function {
                is_generator: _,
                name,
                params,
                docstr: _,
                body,
                varspec,
            } => {
                let short_name = name.clone().unwrap_or_else(|| "<lambda>".into());
                let name = format!("{}#{}", self.name, short_name).into();

                let mut func_builder = Builder::new(Type::Function, name, varspec.clone().unwrap());
                func_builder.expr(body, true)?;
                let func_code = func_builder.build();

                let mut freevar_binding_slots = Vec::new();
                for (freevar, _) in func_code.varspec().free() {
                    let variable = self.varspec.get(freevar).unwrap();
                    assert_eq!(variable.type_(), VariableType::Upval);
                    freevar_binding_slots.push(variable.slot());
                }

                let desc = NewFunctionDesc {
                    code: func_code.into(),
                    argspec: params.clone().into(),
                    freevar_binding_slots,
                };
                self.add(Opcode::NewFunction(desc.into()), mark);
            }
            desc => panic!("TODO compile {:?}", desc),
        }
        Ok(())
    }
    fn args(&mut self, args: &Args) -> Result<()> {
        for arg in &args.args {
            self.expr(arg, true)?;
        }
        for (_, arg) in &args.kwargs {
            self.expr(arg, true)?;
        }
        Ok(())
    }
    fn target(&mut self, target: &AssignTarget, consume: bool) -> Result<()> {
        let mark = target.mark().clone();
        match target.desc() {
            AssignTargetDesc::Name(name) => {
                let variable = self.varspec.get(name).unwrap();
                if consume {
                    self.add(Opcode::SetVar(variable.into()), mark);
                } else {
                    self.add(Opcode::TeeVar(variable.into()), mark);
                }
            }
            AssignTargetDesc::List(targets) => {
                if !consume {
                    self.add(Opcode::Dup, mark.clone());
                }
                self.add(Opcode::Unpack(targets.len() as u32), mark);
                for target in targets.iter().rev() {
                    self.target(target, true)?;
                }
            }
            AssignTargetDesc::Attr(owner, attr) => {
                self.expr(owner, true)?;
                if consume {
                    self.add(Opcode::SetAttr(attr.clone()), mark);
                } else {
                    self.add(Opcode::TeeAttr(attr.clone()), mark);
                }
            }
        }
        Ok(())
    }
    fn here(&self) -> usize {
        self.ops.len()
    }
    fn patch_jump(&mut self, jump_id: usize) {
        let new_dest = self.here();
        self.ops[jump_id].patch_jump(new_dest);
    }
}
