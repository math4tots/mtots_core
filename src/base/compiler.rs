use crate::base::ast::*;
use crate::CallMethodDesc;
use crate::Code;
use crate::Error;
use crate::Mark;
use crate::NewClassDesc;
use crate::NewFunctionDesc;
use crate::Opcode;
use crate::RcStr;
use crate::Result;
use std::collections::HashMap;

const INVALID_JUMP: usize = usize::MAX;

pub fn compile(md: &ModuleDisplay) -> Result<Code> {
    let mut builder = Builder::new(
        Type::Module,
        md.name().clone(),
        md.doc().clone(),
        vec![],
        md.varspec().clone().unwrap(),
    );
    builder.expr(md.body(), true)?;
    Ok(builder.build())
}

enum Type {
    Module,
    Function,
    Generator,
    Async,
}

struct Builder {
    type_: Type,
    name: RcStr,
    params: Vec<Variable>,
    varspec: VarSpec,
    ops: Vec<Opcode>,
    marks: Vec<Mark>,
    doc: Option<RcStr>,
    docmap: HashMap<RcStr, RcStr>,
}

impl Builder {
    fn new(
        type_: Type,
        name: RcStr,
        doc: Option<RcStr>,
        params: Vec<Variable>,
        varspec: VarSpec,
    ) -> Self {
        Self {
            type_,
            name,
            doc,
            params,
            varspec,
            ops: vec![],
            marks: vec![],
            docmap: HashMap::new(),
        }
    }
    fn build(self) -> Code {
        assert_eq!(self.ops.len(), self.marks.len());
        Code::new(
            self.name,
            self.ops,
            self.params,
            self.varspec,
            self.marks,
            self.doc,
            self.docmap,
        )
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
            ExprDesc::List(exprs) => {
                for item in exprs {
                    self.expr(item, true)?;
                }
                if used {
                    self.add(Opcode::NewList(exprs.len() as u32), mark);
                }
            }
            ExprDesc::Map(pairs) => {
                for (key, val) in pairs {
                    self.expr(key, true)?;
                    self.expr(val, true)?;
                }
                self.add(Opcode::NewMap(pairs.len() as u32), mark.clone());
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
            ExprDesc::Switch(valexpr, pairs, default) => {
                // In the future, we may detect special cases and speed things up,
                // but for now switch statements are basically syntactic sugar for
                // if-else chains
                self.expr(valexpr, true)?;
                let mut end_jumps = Vec::new();
                let mut last = None;
                for (cases, body) in pairs {
                    if let Some(last) = last {
                        self.patch_jump(last);
                    }
                    let mut jumps_to_body = Vec::new();
                    for case in cases {
                        self.add(Opcode::Dup, mark.clone());
                        self.expr(case, true)?;
                        self.add(Opcode::Binop(Binop::Eq), mark.clone());
                        jumps_to_body
                            .push(self.add(Opcode::JumpIfTrue(INVALID_JUMP), mark.clone()));
                    }
                    last = Some(self.add(Opcode::Jump(INVALID_JUMP), mark.clone()));
                    for i in jumps_to_body {
                        self.patch_jump(i);
                    }
                    self.expr(body, used)?;
                    end_jumps.push(self.add(Opcode::Jump(INVALID_JUMP), mark.clone()));
                }
                if let Some(last) = last {
                    self.patch_jump(last);
                }
                if let Some(default) = default {
                    self.expr(default, used)?;
                } else if used {
                    self.add(Opcode::Nil, mark.clone());
                }
                for id in end_jumps {
                    self.patch_jump(id);
                }
                // remove the switch comparison value
                if used {
                    self.add(Opcode::Swap01, mark.clone());
                }
                self.add(Opcode::Pop, mark.clone());
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
            ExprDesc::For(target, container, body) => {
                self.expr(container, true)?;
                self.add(Opcode::Iter, mark.clone());
                let start_label = self.len();
                self.add(Opcode::Next, mark.clone());
                let end_jump_id = self.add(Opcode::JumpIfFalse(INVALID_JUMP), mark.clone());
                self.target(target, true)?;
                self.expr(body, false)?;
                self.add(Opcode::Jump(start_label), mark.clone());
                self.patch_jump(end_jump_id);
                if used {
                    // pop the exhausted container while retaining the final
                    // return value from the generator
                    self.add(Opcode::Swap01, mark.clone());
                    self.add(Opcode::Pop, mark);
                } else {
                    // pop both the exhausted container and the returned value
                    self.add(Opcode::Pop, mark.clone());
                    self.add(Opcode::Pop, mark);
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
                self.add(Opcode::Binop(*op), mark.clone());
                if !used {
                    self.add(Opcode::Pop, mark);
                }
            }
            ExprDesc::LogicalBinop(op, lhs, rhs) => {
                self.expr(lhs, true)?;
                let jump_id = match op {
                    LogicalBinop::And => {
                        self.add(Opcode::TeeJumpIfFalse(INVALID_JUMP), mark.clone())
                    }
                    LogicalBinop::Or => self.add(Opcode::TeeJumpIfTrue(INVALID_JUMP), mark.clone()),
                };
                self.expr(rhs, true)?;
                self.patch_jump(jump_id);
                if !used {
                    self.add(Opcode::Pop, mark);
                }
            }
            ExprDesc::Unop(op, arg) => {
                self.expr(arg, true)?;
                self.add(Opcode::Unop(*op), mark.clone());
                if !used {
                    self.add(Opcode::Pop, mark);
                }
            }
            ExprDesc::Subscript(owner, index) => {
                self.expr(owner, true)?;
                self.expr(index, true)?;
                self.add(Opcode::GetItem, mark.clone());
                if !used {
                    self.add(Opcode::Pop, mark);
                }
            }
            ExprDesc::Slice(owner, start, end) => {
                self.expr(owner, true)?;
                if let Some(start) = start {
                    self.expr(start, true)?;
                } else {
                    self.add(Opcode::Nil, mark.clone());
                }
                if let Some(end) = end {
                    self.expr(end, true)?;
                } else {
                    self.add(Opcode::Nil, mark.clone());
                }
                self.add(
                    Opcode::CallMethod(
                        CallMethodDesc {
                            argc: 2,
                            kwargs: vec![],
                            method_name: "__slice".into(),
                        }
                        .into(),
                    ),
                    mark.clone(),
                );
                if !used {
                    self.add(Opcode::Pop, mark);
                }
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
            ExprDesc::CallMethod(owner, method_name, args) => {
                self.expr(owner, true)?;
                self.args(args)?;
                let info = args.call_method_info(method_name.clone());
                self.add(Opcode::CallMethod(info.into()), mark.clone());
                if !used {
                    self.add(Opcode::Pop, mark);
                }
            }
            ExprDesc::Assign(target, valexpr) => {
                self.expr(valexpr, true)?;
                self.target(target, !used)?;
            }
            ExprDesc::AugAssign(target, op, valexpr) => {
                self.expr(valexpr, true)?;
                self.augtarget(target, *op, !used)?;
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
            ExprDesc::Nonlocal(_names) => {
                if used {
                    self.add(Opcode::Nil, mark);
                }
            }
            ExprDesc::New(hidden_class_name, kwargs) => {
                // load the associated class object
                let variable = self
                    .varspec
                    .get(hidden_class_name.as_ref().unwrap())
                    .unwrap();
                self.add(Opcode::GetVar(variable.into()), mark.clone());

                // load all the arguments
                let mut names = Vec::new();
                for (name, arg) in kwargs {
                    names.push(name.clone());
                    self.expr(arg, true)?;
                }

                // create the new Table object
                self.add(Opcode::New(names.into()), mark.clone());

                if !used {
                    self.add(Opcode::Pop, mark);
                }
            }
            ExprDesc::Del(name) => {
                let variable = self.varspec.get(name).unwrap();
                self.add(Opcode::Del(variable.into()), mark.clone());
                if !used {
                    self.add(Opcode::Pop, mark);
                }
            }
            ExprDesc::Yield(valexpr) => {
                match self.type_ {
                    Type::Module | Type::Function | Type::Async => {
                        return Err(Error::rt(
                            format!("Yield is not allowed here").into(),
                            vec![mark],
                        ));
                    }
                    Type::Generator => {}
                }
                self.expr(valexpr, true)?;
                self.add(Opcode::Yield, mark.clone());
                if !used {
                    self.add(Opcode::Pop, mark);
                }
            }
            ExprDesc::Await(valexpr) => {
                match self.type_ {
                    Type::Module | Type::Function | Type::Generator => {
                        return Err(Error::rt(
                            format!("Await is not allowed here").into(),
                            vec![mark],
                        ));
                    }
                    Type::Async => {}
                }
                self.expr(valexpr, true)?;
                self.add(Opcode::Await, mark.clone());
                if !used {
                    self.add(Opcode::Pop, mark);
                }
            }
            ExprDesc::Return(valexpr) => {
                match self.type_ {
                    Type::Module => {
                        return Err(Error::rt(
                            format!("Return is not allowed here").into(),
                            vec![mark],
                        ));
                    }
                    Type::Function | Type::Generator | Type::Async => {}
                }
                if let Some(valexpr) = valexpr {
                    self.expr(valexpr, true)?;
                } else {
                    self.add(Opcode::Nil, mark.clone());
                }
                self.add(Opcode::Return, mark);
            }
            ExprDesc::Import(name) => {
                let name = if name.starts_with('.') {
                    let mut depth = 1;
                    let mut prefix = self.name.str();
                    while name[depth..].starts_with('.') {
                        depth += 1;
                        if let Some(i) = prefix.rfind('.') {
                            prefix = &prefix[..i];
                        } else {
                            return Err(Error::rt(
                                format!(
                                    concat!(
                                        "{:?} requires at least {} level of unwraping ",
                                        "but {:?} is not that far nested",
                                    ),
                                    name, depth, self.name,
                                )
                                .into(),
                                vec![],
                            ));
                        }
                    }
                    format!("{}.{}", prefix, &name[depth..]).into()
                } else {
                    name.clone()
                };
                self.add(Opcode::Import(name), mark.clone());
                if !used {
                    self.add(Opcode::Pop, mark);
                }
            }
            ExprDesc::BreakPoint => {
                self.add(Opcode::Breakpoint, mark.clone());
                if used {
                    self.add(Opcode::Nil, mark);
                }
            }
            ExprDesc::GetCallingModule => {
                if used {
                    self.add(Opcode::GetCallingModule, mark);
                }
            }
            ExprDesc::AssignDoc(expr, name, doc) => {
                self.docmap.insert(name.clone(), doc.clone());
                self.expr(expr, used)?;
            }
            ExprDesc::Function {
                kind,
                name,
                params,
                docstr,
                body,
                varspec,
            } => {
                let varspec = varspec.as_ref().unwrap().clone();
                let short_name = name.clone().unwrap_or_else(|| "<lambda>".into());
                let name = format!("{}#{}", self.name, short_name).into();

                let param_vars = {
                    let mut vars = Vec::new();
                    for name in params.params() {
                        vars.push(varspec.get(&name).unwrap());
                    }
                    vars
                };

                let type_ = match *kind {
                    FunctionKind::Normal => Type::Function,
                    FunctionKind::Generator => Type::Generator,
                    FunctionKind::Async => Type::Async,
                };
                let mut func_builder =
                    Builder::new(type_, name, docstr.clone(), param_vars, varspec);
                match *kind {
                    FunctionKind::Generator => {
                        // The first resume on a generator will push a value
                        // on the stack before the generator has had a chance to start.
                        // We ignore this value by always popping at the beginning
                        // of every generator
                        func_builder.add(Opcode::Pop, mark.clone());
                    }
                    _ => {}
                }
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
                    kind: *kind,
                };
                self.add(Opcode::NewFunction(desc.into()), mark.clone());

                if !used {
                    self.add(Opcode::Pop, mark);
                }
            }
            ExprDesc::Class {
                name,
                bases,
                docstr: _,
                methods,
                static_methods,
                hidden_name,
            } => {
                let name = RcStr::from(format!("{}#{}", self.name, name));
                for base in bases {
                    self.expr(base, true)?;
                }
                let mut method_names = Vec::new();
                for (name, method) in methods {
                    method_names.push(name.clone());
                    self.expr(method, true)?;
                }
                let mut static_method_names = Vec::new();
                for (name, method) in static_methods {
                    static_method_names.push(name.clone());
                    self.expr(method, true)?;
                }
                let desc = NewClassDesc {
                    name,
                    nbases: bases.len(),
                    method_names,
                    static_method_names,
                };
                self.add(Opcode::NewClass(desc.into()), mark.clone());
                let hidden_class_var = self.varspec.get(hidden_name.as_ref().unwrap()).unwrap();
                self.add(Opcode::TeeVar(hidden_class_var.into()), mark.clone());

                if !used {
                    self.add(Opcode::Pop, mark);
                }
            }
        }
        Ok(())
    }
    fn args(&mut self, args: &Args) -> Result<()> {
        for arg in &args.args {
            self.expr(arg, true)?;
        }
        if let Some(arg) = &args.varargs {
            self.expr(arg, true)?;
        }
        for (_, arg) in &args.kwargs {
            self.expr(arg, true)?;
        }
        if let Some(arg) = &args.kwmap {
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
            AssignTargetDesc::Subscript(owner, index) => {
                self.expr(owner, true)?;
                self.expr(index, true)?;
                if consume {
                    self.add(Opcode::SetItem, mark);
                } else {
                    self.add(Opcode::TeeItem, mark);
                }
            }
        }
        Ok(())
    }
    fn augtarget(&mut self, target: &AssignTarget, op: Binop, consume: bool) -> Result<()> {
        let mark = target.mark().clone();
        match target.desc() {
            AssignTargetDesc::Name(name) => {
                let variable = self.varspec.get(name).unwrap();
                self.add(Opcode::GetVar(variable.clone().into()), mark.clone());
                self.add(Opcode::Swap01, mark.clone());
                self.add(Opcode::Binop(op), mark.clone());
                if consume {
                    self.add(Opcode::SetVar(variable.into()), mark);
                } else {
                    self.add(Opcode::TeeVar(variable.into()), mark);
                }
            }
            AssignTargetDesc::List(_) => {
                // Should be caught in the annotator
                panic!("List pattern as augassign target")
            }
            AssignTargetDesc::Attr(owner, attr) => {
                self.expr(owner, true)?;
                self.add(Opcode::Dup, mark.clone());
                self.add(Opcode::GetAttr(attr.clone()), mark.clone());
                self.add(Opcode::Pull2, mark.clone());
                self.add(Opcode::Binop(op), mark.clone());
                self.add(Opcode::Swap01, mark.clone());
                if consume {
                    self.add(Opcode::SetAttr(attr.clone()), mark);
                } else {
                    self.add(Opcode::TeeAttr(attr.clone()), mark);
                }
            }
            AssignTargetDesc::Subscript(owner, index) => {
                self.expr(owner, true)?;
                self.expr(index, true)?;
                self.add(Opcode::Dup2, mark.clone());
                self.add(Opcode::GetItem, mark.clone());
                self.add(Opcode::Pull3, mark.clone());
                self.add(Opcode::Binop(op), mark.clone());
                self.add(Opcode::Unpull2, mark.clone());
                if consume {
                    self.add(Opcode::SetItem, mark.clone());
                } else {
                    self.add(Opcode::TeeItem, mark.clone());
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
