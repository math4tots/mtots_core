use crate::ArgumentList;
use crate::Binop;
use crate::Code;
use crate::CodeBuilder;
use crate::CodeBuilderError;
use crate::ConstValue;
use crate::Expression;
use crate::ExpressionData;
use crate::ExpressionKind;
use crate::ParameterInfo;
use crate::RcStr;
use crate::SymbolRegistryHandle;
use crate::Unop;
use crate::Value;

use std::fmt;

#[derive(Debug)]
pub struct CompileError {
    name: RcStr,
    lineno: usize,
    kind: CompileErrorKind,
}

impl CompileError {
    pub fn move_(self) -> (RcStr, usize, CompileErrorKind) {
        (self.name, self.lineno, self.kind)
    }
}

#[derive(Debug)]
pub enum CompileErrorKind {
    InvalidAssignmentTarget(ExpressionKind),
    ExpectedConstantExpression(ExpressionKind),
    InvalidRelativeImportError {
        module_name: String,
        import_path: String,
    },
}

impl fmt::Display for CompileErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompileErrorKind::InvalidAssignmentTarget(target_kind) => {
                write!(f, "{:?} is not assignable", target_kind)?;
            }
            CompileErrorKind::ExpectedConstantExpression(kind) => {
                write!(f, "Expected a constant expression but got {:?}", kind)?;
            }
            CompileErrorKind::InvalidRelativeImportError
        }
        Ok(())
    }
}

struct Error {
    lineno: usize,
    kind: CompileErrorKind,
}

impl Error {
    fn new(lineno: usize, kind: CompileErrorKind) -> Error {
        Error { lineno, kind }
    }
}
impl From<CodeBuilderError> for Error {
    fn from(err: CodeBuilderError) -> Error {
        match err {}
    }
}

pub fn compile(
    symbol_registry: SymbolRegistryHandle,
    name: RcStr,
    expr: &Expression,
) -> Result<Code, CompileError> {
    let mut builder = CodeBuilder::for_module(symbol_registry, name.clone());
    let result = (|| -> Result<Code, Error> {
        rec(&mut builder, expr, true)?;
        Ok(builder.build()?)
    })();
    match result {
        Ok(code) => Ok(code),
        Err(error) => Err(CompileError {
            name,
            lineno: error.lineno,
            kind: error.kind,
        }),
    }
}

/// compiles the given expression
/// the 'used' argument indicates whether the result of the expression is
/// actually used, and determines whether the evaluated value should
/// remain on the stack after the expression finishes evaluating.
fn rec(builder: &mut CodeBuilder, expr: &Expression, used: bool) -> Result<(), Error> {
    match expr.data() {
        ExpressionData::Nil => {
            if used {
                builder.load_const(())
            }
        }
        ExpressionData::Bool(x) => {
            if used {
                builder.load_const(*x)
            }
        }
        ExpressionData::Int(x) => {
            if used {
                builder.load_const(*x)
            }
        }
        ExpressionData::Float(x) => {
            if used {
                builder.load_const(*x)
            }
        }
        ExpressionData::Symbol(x) => {
            if used {
                builder.load_const(*x)
            }
        }
        ExpressionData::String(x) => {
            if used {
                builder.load_const(x.clone())
            }
        }
        ExpressionData::MutableString(x) => {
            if used {
                builder.make_mutable_string(x);
            }
        }
        ExpressionData::Name(x) => {
            // for variables, we need to load them regardless of whether
            // the value is actually used, since the act of loading itself could have
            // side-effects
            builder.load_var(x.clone());
            if !used {
                builder.pop();
            }
        }
        ExpressionData::Del(x) => {
            // Delete the given variable.
            // Note that, if used as an expression, we expect to return the old
            // value that used to be there.
            // This feature is important for functions that expect unique
            // references (e.g. __raise)
            if used {
                builder.load_var(x.clone());
            }
            builder.load_const(ConstValue::Uninitialized);
            builder.store_var(x.clone());
        }
        ExpressionData::Nonlocal(names) => {
            for name in names {
                builder.nonlocal(name.clone());
            }
            if used {
                builder.load_const(());
            }
        }
        ExpressionData::Parentheses(expr) => rec(builder, expr, used)?,
        ExpressionData::Block(exprs) => {
            if exprs.is_empty() {
                if used {
                    builder.load_const(());
                }
            } else {
                let mut exprs = exprs.iter().peekable();
                while let Some(expr) = exprs.next() {
                    builder.lineno(expr.lineno());
                    let rec_used = if exprs.peek().is_some() { false } else { used };
                    rec(builder, expr, rec_used)?;
                }
            }
        }
        ExpressionData::ListDisplay(exprs) => {
            for expr in exprs {
                rec(builder, expr, used)?;
            }
            if used {
                builder.make_list(exprs.len());
            }
        }
        ExpressionData::MapDisplay(pairs) => {
            for (key, val) in pairs {
                rec(builder, key, used)?;
                rec(builder, val, used)?;
            }
            if used {
                builder.make_map(pairs.len());
            }
        }
        ExpressionData::MutableListDisplay(exprs) => {
            for expr in exprs {
                rec(builder, expr, used)?;
            }
            if used {
                builder.make_mutable_list(exprs.len());
            }
        }
        ExpressionData::MutableMapDisplay(pairs) => {
            for (key, val) in pairs {
                rec(builder, key, used)?;
                rec(builder, val, used)?;
            }
            if used {
                builder.make_mutable_map(pairs.len());
            }
        }
        ExpressionData::Assign(target, expr) => {
            // You'll notice that the RHS gets evaluated first
            // even though the LHS appears first in the source text.
            // But Python also behaves this way, so I'm gonna say this is OK.
            rec(builder, expr, true)?;
            if used {
                builder.dup_top();
            }
            assign(builder, target)?;
        }
        ExpressionData::If(pairs, other) => {
            let end = builder.new_label();

            for (cond, body) in pairs {
                let next = builder.new_label();

                rec(builder, cond, true)?;
                builder.pop_jump_if_false(next);

                rec(builder, body, used)?;
                builder.jump(end);

                builder.label(next);
            }

            if let Some(other) = other {
                rec(builder, other, used)?;
            } else if used {
                builder.load_const(());
            }

            builder.label(end);
        }
        ExpressionData::For(target, iterable, body) => {
            let start = builder.new_label();
            let end = builder.new_label();

            rec(builder, iterable, true)?;
            builder.get_iter(); // TOS = iter(TOS)
            builder.label(start);
            builder.for_iter(end); // push(next(TOS)) or pop and jump to END if done
            assign(builder, target)?;
            rec(builder, body, false)?;
            builder.jump(start);
            builder.label(end);
            if used {
                builder.load_const(());
            }
        }
        ExpressionData::While(cond, body) => {
            let start = builder.new_label();
            let end = builder.new_label();

            builder.label(start);
            rec(builder, cond, true)?;
            builder.pop_jump_if_false(end);
            rec(builder, body, false)?;
            builder.jump(start);
            builder.label(end);
            if used {
                builder.load_const(());
            }
        }
        ExpressionData::Unop(op, expr) => {
            rec(builder, expr, true)?;
            builder.unop(*op);
            if !used {
                builder.pop();
            }
        }
        ExpressionData::Binop(op, lhs, rhs) => {
            match op {
                Binop::And => {
                    let end = builder.new_label();
                    rec(builder, lhs, true)?;
                    builder.jump_if_false_or_pop(end);
                    rec(builder, rhs, true)?;
                    builder.label(end);
                    if !used {
                        builder.pop();
                    }
                }
                Binop::Or => {
                    let end = builder.new_label();
                    rec(builder, lhs, true)?;
                    builder.jump_if_true_or_pop(end);
                    rec(builder, rhs, true)?;
                    builder.label(end);
                    if !used {
                        builder.pop();
                    }
                }
                Binop::IsNot => {
                    rec(builder, lhs, true)?;
                    rec(builder, rhs, true)?;
                    builder.binop(Binop::Is);
                    builder.unop(Unop::Not);
                    if !used {
                        builder.pop();
                    }
                }
                Binop::Ne => {
                    rec(builder, lhs, true)?;
                    rec(builder, rhs, true)?;
                    builder.binop(Binop::Eq);
                    builder.unop(Unop::Not);
                    if !used {
                        builder.pop();
                    }
                }
                Binop::Gt => {
                    // a > b <=> b < a
                    rec(builder, lhs, true)?;
                    rec(builder, rhs, true)?;
                    builder.rot_two();
                    builder.binop(Binop::Lt);
                    if !used {
                        builder.pop();
                    }
                }
                Binop::Le => {
                    // a <= b <=> !(b < a)
                    rec(builder, lhs, true)?;
                    rec(builder, rhs, true)?;
                    builder.rot_two();
                    builder.binop(Binop::Lt);
                    builder.unop(Unop::Not);
                    if !used {
                        builder.pop();
                    }
                }
                Binop::Ge => {
                    // a >= b <=> !(a < b)
                    rec(builder, lhs, true)?;
                    rec(builder, rhs, true)?;
                    builder.binop(Binop::Lt);
                    builder.unop(Unop::Not);
                    if !used {
                        builder.pop();
                    }
                }
                _ => {
                    rec(builder, lhs, true)?;
                    rec(builder, rhs, true)?;
                    builder.binop(*op);
                    if !used {
                        builder.pop();
                    }
                }
            }
        }
        ExpressionData::Attribute(owner, name) => {
            rec(builder, owner, true)?;
            builder.load_attr(name);
            if !used {
                builder.pop();
            }
        }
        ExpressionData::StaticAttribute(owner, name) => {
            rec(builder, owner, true)?;
            builder.load_static_attr(name);
            if !used {
                builder.pop();
            }
        }
        ExpressionData::Subscript(owner, index) => {
            rec(builder, owner, true)?;
            builder.load_method(&"__getitem".into());
            finish_call0(builder, true, vec![&*index])?;
            if !used {
                builder.pop();
            }
        }
        ExpressionData::FunctionCall(f, arglist) => {
            rec(builder, f, true)?;
            finish_call(builder, false, arglist)?;
            if !used {
                builder.pop();
            }
        }
        ExpressionData::MethodCall(owner, name, arglist) => {
            rec(builder, owner, true)?;
            builder.load_method(name);
            finish_call(builder, true, arglist)?;
            if !used {
                builder.pop();
            }
        }
        ExpressionData::FunctionDisplay(is_generator, short_name, req, opt, var, kw, body) => {
            let sr = builder.symbol_registry().clone();
            let short_name = match short_name {
                Some(short_name) => short_name.clone(),
                None => "<lambda>".into(),
            };
            let lineno = expr.lineno();
            let req = req.iter().map(|s| builder.intern_rcstr(s)).collect();
            let opt = {
                let mut pairs = Vec::new();
                for (name, expr) in opt {
                    pairs.push((builder.intern_rcstr(name), Value::from(consteval(expr)?)));
                }
                pairs
            };
            let var = var.as_ref().map(|s| builder.intern_rcstr(s));
            let kw = kw.as_ref().map(|s| builder.intern_rcstr(s));
            let parameter_info = ParameterInfo::new(req, opt, var, kw);
            let full_func_name = format!("{}.{}", builder.full_name(), short_name);
            let mut func_builder = if *is_generator {
                CodeBuilder::for_generator(
                    builder.symbol_registry().clone(),
                    parameter_info,
                    builder.module_name().clone(),
                    full_func_name.into(),
                    lineno,
                )
            } else {
                CodeBuilder::for_func(
                    builder.symbol_registry().clone(),
                    parameter_info,
                    builder.module_name().clone(),
                    full_func_name.into(),
                    lineno,
                )
            };
            rec(&mut func_builder, body, true)?;
            let func_code = func_builder.build()?;

            for freevar in func_code.freevars() {
                builder.load_cell(sr.rcstr(*freevar).clone());
            }
            builder.make_list(func_code.freevars().len());

            let func_code_index = builder.add_code_obj(func_code.into());
            builder.make_func(func_code_index);

            if !used {
                builder.pop();
            }
        }
        ExpressionData::ClassDisplay(
            is_trait,
            short_name,
            bases,
            _docstring,
            fields,
            methods,
            static_methods,
        ) => {
            for base in bases {
                rec(builder, base, true)?;
            }
            builder.make_list(bases.len());

            let fields = if *is_trait {
                assert!(fields.is_none());
                vec![]
            } else {
                if let Some(fields) = fields {
                    fields.clone()
                } else {
                    vec![]
                }
            };
            for field in &fields {
                builder.load_const(*field);
            }
            builder.make_list(fields.len());

            for (keystr, method) in methods {
                let key = builder.intern_rcstr(keystr);
                builder.load_const(key);
                rec(builder, method, true)?;
            }
            builder.make_table(methods.len());

            for (keystr, method) in static_methods {
                let key = builder.intern_rcstr(keystr);
                builder.load_const(key);
                rec(builder, method, true)?;
            }
            builder.make_table(static_methods.len());

            let full_name = format!("{}::{}", builder.module_name(), short_name).into();
            builder.make_class(&full_name, *is_trait);

            if !used {
                builder.pop();
            }
        }
        ExpressionData::ExceptionKindDisplay(short_name, base, _docstring, fields, template) => {
            let full_name = format!("{}::{}", builder.module_name(), short_name).into();

            if let Some(base) = base {
                rec(builder, base, true)?;
            } else {
                builder.load_const(());
            }

            if let Some(fields) = fields {
                for field in fields {
                    builder.load_const(*field);
                }
                builder.make_list(fields.len());
            } else {
                builder.load_const(());
            }

            rec(builder, template, true)?;

            builder.make_exception_kind(&full_name);

            if !used {
                builder.pop();
            }
        }
        ExpressionData::Import(name) => {
            if name.starts_with('.') {
                let mut module_name = builder.module_name().str().to_owned();
                let up_cnt = name.matches('.').count() - 1;
                for i in 0..up_cnt {
                    if let Some(i) = module_name.rfind('.') {
                        module_name.truncate(i);
                    } else {

                    }
                }
                builder.import_(&format!(
                    "{}{}",
                    builder.module_name(),
                    name,
                ).into())
            } else {
                builder.import_(name);
            }
        }
        ExpressionData::Yield(expr) => {
            rec(builder, expr, true)?;
            builder.yield_();
            if !used {
                builder.pop();
            }
        }
        ExpressionData::Return(expr) => {
            if let Some(expr) = expr {
                rec(builder, expr, true)?;
            } else {
                builder.load_const(());
            }
            builder.return_();
        }
        ExpressionData::BreakPoint => {
            builder.breakpoint();
            if used {
                builder.load_const(());
            }
        }
    }
    Ok(())
}

fn consteval(expr: &Expression) -> Result<ConstValue, Error> {
    Ok(match expr.data() {
        ExpressionData::Nil => ConstValue::Nil,
        ExpressionData::Bool(x) => ConstValue::Bool(*x),
        ExpressionData::Int(x) => ConstValue::Int(*x),
        ExpressionData::Float(x) => ConstValue::Float(x.to_bits()),
        ExpressionData::String(x) => ConstValue::String(x.clone()),
        _ => {
            return Err(Error::new(
                expr.lineno(),
                CompileErrorKind::ExpectedConstantExpression(expr.kind()),
            ));
        }
    })
}

fn assign(builder: &mut CodeBuilder, target: &Expression) -> Result<(), Error> {
    match target.data() {
        ExpressionData::Name(name) => {
            builder.store_var(name.clone());
        }
        ExpressionData::Attribute(owner, name) => {
            rec(builder, owner, true)?;
            builder.store_attr(name);
        }
        ExpressionData::ListDisplay(subtargets) => {
            builder.unpack(subtargets.len());
            for subtarget in subtargets {
                assign(builder, subtarget)?;
            }
        }
        ExpressionData::Subscript(owner, index) => {
            rec(builder, owner, true)?;
            builder.load_method(&"__setitem".into());
            rec(builder, index, true)?;

            // at this point, the stack looks like:
            //      TOS : index
            //      TOS1: owner
            //      TOS2: method
            //      TOS3: expr
            // we use pull_tos3, fix the order for the method call
            builder.pull_tos3();
            builder.call_func(3);

            builder.pop();
        }
        _ => {
            return Err(Error::new(
                target.lineno(),
                CompileErrorKind::InvalidAssignmentTarget(target.kind()),
            ));
        }
    }
    Ok(())
}

fn finish_call(
    builder: &mut CodeBuilder,
    account_for_owner: bool,
    arglist: &ArgumentList,
) -> Result<(), Error> {
    if let Some(args) = arglist.trivial() {
        for arg in args {
            rec(builder, arg, true)?;
        }
        builder.call_func(if account_for_owner { 1 } else { 0 } + args.len());
        Ok(())
    } else {
        let args = arglist.positional();
        let kwargs = arglist.keyword();
        let variadic = arglist.variadic();
        let table = arglist.table();

        // positional args
        for arg in args {
            rec(builder, arg, true)?;
        }
        builder.make_list(if account_for_owner { 1 } else { 0 } + args.len());

        // keyword args
        for (keystr, arg) in kwargs {
            let key = builder.intern_rcstr(keystr);
            builder.load_const(key);
            rec(builder, arg, true)?;
        }
        builder.make_table(kwargs.len());

        // variadic (extra positional args)
        if let Some(variadic) = variadic {
            builder.rot_two(); // bring the arg vec to TOS
            rec(builder, variadic, true)?;
            builder.extend_list();
            builder.rot_two(); // bring the kw table back to TOS
        }

        // table args (extra keyword args)
        if let Some(table) = table {
            rec(builder, table, true)?;
            builder.extend_table();
        }

        builder.call_func_generic();
        Ok(())
    }
}

fn finish_call0(
    builder: &mut CodeBuilder,
    account_for_owner: bool,
    args: Vec<&Expression>,
) -> Result<(), Error> {
    for arg in &args {
        rec(builder, arg, true)?;
    }
    builder.call_func(if account_for_owner { 1 } else { 0 } + args.len());
    Ok(())
}
