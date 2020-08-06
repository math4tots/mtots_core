use crate::ArgSpec;
use crate::Args;
use crate::AssignTarget;
use crate::AssignTargetDesc;
use crate::Expr;
use crate::ExprDesc;
use crate::Mark;
use crate::ModuleDisplay;
use crate::RcStr;
use crate::Result;
use crate::VarSpec;
use std::collections::HashMap;

/// Annotate a module and all functions in it with variable information
pub fn annotate(md: &mut ModuleDisplay) -> Result<()> {
    let mut out = State {
        type_: Type::Module,
        ..State::default()
    };
    get(md.body_mut(), &mut out)?;
    *md.varspec_mut() = Some(out.into());
    Ok(())
}

fn annotate_func(params: &ArgSpec, body: &mut Expr) -> Result<VarSpec> {
    let mut out = State {
        type_: Type::Function,
        ..State::default()
    };
    for param in params.params() {
        out.write.insert(param, body.mark().clone());
    }
    get(body, &mut out)?;
    Ok(out.into())
}

fn get(expr: &mut Expr, out: &mut State) -> Result<()> {
    let mark = expr.mark().clone();
    match expr.desc_mut() {
        ExprDesc::Nil => {}
        ExprDesc::Bool(_) => {}
        ExprDesc::Number(_) => {}
        ExprDesc::String(_) => {}
        ExprDesc::Name(name) => {
            if !out.read.contains_key(name) {
                out.read.insert(name.clone(), mark);
            }
        }
        ExprDesc::List(exprs) => {
            for expr in exprs {
                get(expr, out)?;
            }
        }
        ExprDesc::Map(pairs) => {
            for (key, val) in pairs {
                get(key, out)?;
                get(val, out)?;
            }
        }
        ExprDesc::Parentheses(expr) => {
            get(expr, out)?;
        }
        ExprDesc::Block(exprs) => {
            for child in exprs {
                get(child, out)?;
            }
        }
        ExprDesc::If(pairs, other) => {
            for (cond, body) in pairs {
                get(cond, out)?;
                get(body, out)?;
            }
            if let Some(other) = other {
                get(other, out)?;
            }
        }
        ExprDesc::For(target, container, body) => {
            gettarget(target, out)?;
            get(container, out)?;
            get(body, out)?;
        }
        ExprDesc::While(cond, body) => {
            get(cond, out)?;
            get(body, out)?;
        }
        ExprDesc::Binop(_op, lhs, rhs) => {
            get(lhs, out)?;
            get(rhs, out)?;
        }
        ExprDesc::LogicalBinop(_op, lhs, rhs) => {
            get(lhs, out)?;
            get(rhs, out)?;
        }
        ExprDesc::Attr(owner, _attr) => {
            get(owner, out)?;
        }
        ExprDesc::CallFunction(f, args) => {
            get(f, out)?;
            getargs(args, out)?;
        }
        ExprDesc::CallMethod(owner, _name, args) => {
            get(owner, out)?;
            getargs(args, out)?;
        }
        ExprDesc::Assign(target, valexpr) => {
            gettarget(target, out)?;
            get(valexpr, out)?;
        }
        ExprDesc::NonlocalAssign(name, valexpr) => {
            if !out.nonlocal.contains_key(name) {
                out.nonlocal.insert(name.clone(), mark);
            }
            get(valexpr, out)?;
        }
        ExprDesc::Yield(expr) => {
            get(expr, out)?;
        }
        ExprDesc::Return(expr) => {
            if let Some(expr) = expr {
                get(expr, out)?;
            }
        }
        ExprDesc::Import(_) => {}
        ExprDesc::AssignDoc(expr, _name, _doc) => {
            get(expr, out)?;
        }
        ExprDesc::Function {
            is_generator: _,
            name: _,
            params,
            docstr: _,
            body,
            varspec,
        } => {
            let spec = annotate_func(params, body)?;
            for (name, mark) in spec.free() {
                out.nested_free.insert(name.clone(), mark.clone());
            }
            *varspec = Some(spec);
        }
        desc => panic!("TODO: annotate {:?}", desc),
    }
    Ok(())
}

fn getargs(args: &mut Args, out: &mut State) -> Result<()> {
    for arg in &mut args.args {
        get(arg, out)?;
    }
    for (_, arg) in &mut args.kwargs {
        get(arg, out)?;
    }
    Ok(())
}

fn gettarget(target: &mut AssignTarget, out: &mut State) -> Result<()> {
    match target.desc_mut() {
        AssignTargetDesc::Name(name) => {
            if !out.write.contains_key(name) {
                out.write.insert(name.clone(), target.mark().clone());
            }
        }
        AssignTargetDesc::List(targets) => {
            for child in targets {
                gettarget(child, out)?;
            }
        }
        AssignTargetDesc::Attr(owner, _attr) => {
            get(owner, out)?;
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum Type {
    Invalid,
    Module,
    Function,
}

impl Default for Type {
    fn default() -> Self {
        Self::Invalid
    }
}

#[derive(Default)]
struct State {
    type_: Type,
    read: HashMap<RcStr, Mark>,
    write: HashMap<RcStr, Mark>,

    /// Variables marked explicitly as nonlocal
    /// These must be nonlocal upvalues
    nonlocal: HashMap<RcStr, Mark>,

    /// Variables that appear free in nested functions
    /// These must be either owned or free upvalues.
    nested_free: HashMap<RcStr, Mark>,
}

impl From<State> for VarSpec {
    fn from(mut state: State) -> Self {
        let mut local = Vec::new();
        let mut free = Vec::new();
        let mut owned = Vec::new();

        // anything marked nonlocal is always free
        for (name, mark) in state.nonlocal {
            state.read.remove(&name);
            state.write.remove(&name);
            state.nested_free.remove(&name);
            free.push((name, mark));
        }

        // of those remaining,
        // anything written to is always either local or owned
        // (depends on whether it's free in a nested function)
        for (name, mark) in state.write {
            state.read.remove(&name);
            if let Some(_) = state.nested_free.remove(&name) {
                owned.push((name, mark));
            } else {
                // For normal functions, variables at this point
                // would be local.
                // For modules however, we want it to be an owned
                // variable so that its cell can be passed around
                match state.type_ {
                    Type::Invalid => panic!("annotator::State::from(Type::Invalid)"),
                    Type::Module => owned.push((name, mark)),
                    Type::Function => local.push((name, mark)),
                }
            }
        }

        // all remaining variables are free
        for (name, mark) in state.read {
            state.nested_free.remove(&name);
            free.push((name, mark));
        }
        free.extend(state.nested_free);

        Self::new(local, free, owned)
    }
}
