use crate::ArgSpec;
use crate::Args;
use crate::AssignTarget;
use crate::AssignTargetDesc;
use crate::Error;
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

fn annotate_func(class_stack: Vec<RcStr>, params: &ArgSpec, body: &mut Expr) -> Result<VarSpec> {
    let mut out = State {
        type_: Type::Function,
        class_stack,
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
        ExprDesc::Switch(valexpr, pairs, default) => {
            get(valexpr, out)?;
            for (case, body) in pairs {
                get(case, out)?;
                get(body, out)?;
            }
            if let Some(default) = default {
                get(default, out)?;
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
        ExprDesc::Unop(_op, expr) => {
            get(expr, out)?;
        }
        ExprDesc::Subscript(owner, index) => {
            get(owner, out)?;
            get(index, out)?;
        }
        ExprDesc::Slice(owner, start, end) => {
            get(owner, out)?;
            if let Some(start) = start {
                get(start, out)?;
            }
            if let Some(end) = end {
                get(end, out)?;
            }
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
        ExprDesc::AugAssign(target, _op, valexpr) => {
            getaugtarget(target, out)?;
            get(valexpr, out)?;
        }
        ExprDesc::NonlocalAssign(name, valexpr) => {
            if !out.nonlocal.contains_key(name) {
                out.nonlocal.insert(name.clone(), mark);
            }
            get(valexpr, out)?;
        }
        ExprDesc::Nonlocal(names) => {
            for name in names {
                if !out.nonlocal.contains_key(name) {
                    out.nonlocal.insert(name.clone(), mark.clone());
                }
            }
        }
        ExprDesc::New(hidden_class_name, pairs) => {
            if let Some(hidden_name) = out.class_stack.last() {
                out.read.insert(hidden_name.clone(), mark);
                *hidden_class_name = Some(hidden_name.clone());
            } else {
                return Err(Error::rt(
                    format!("The new operator cannot be used outside of a class").into(),
                    vec![mark],
                ));
            }
            for (_, expr) in pairs {
                get(expr, out)?;
            }
        }
        ExprDesc::Del(name) => {
            if !out.write.contains_key(name) {
                out.write.insert(name.clone(), mark);
            }
        }
        ExprDesc::Yield(expr) => {
            get(expr, out)?;
        }
        ExprDesc::Await(expr) => {
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
            kind: _,
            name: _,
            params,
            docstr: _,
            body,
            varspec,
        } => {
            let spec = annotate_func(out.class_stack.clone(), params, body)?;
            for (name, mark) in spec.free() {
                out.nested_free.insert(name.clone(), mark.clone());
            }
            *varspec = Some(spec);
        }
        ExprDesc::Class {
            name,
            bases,
            docstr: _,
            methods,
            static_methods,
            hidden_name,
        } => {
            for base in bases {
                get(base, out)?;
            }

            // Even if a class is never explicitly saved to a variable,
            // we save it to a hidden local variable so that it can be referred
            // to by its own methods.
            let id = out.class_stack.len();
            let qualified_hidden_name = RcStr::from(format!("class/{}/{}", id, name));
            out.write.insert(qualified_hidden_name.clone(), mark);
            *hidden_name = Some(qualified_hidden_name.clone());
            out.class_stack.push(qualified_hidden_name);

            for (_, expr) in methods {
                get(expr, out)?;
            }
            for (_, expr) in static_methods {
                get(expr, out)?;
            }

            out.class_stack.pop();
        }
        desc => panic!("TODO: annotate {:?}", desc),
    }
    Ok(())
}

fn getargs(args: &mut Args, out: &mut State) -> Result<()> {
    for arg in &mut args.args {
        get(arg, out)?;
    }
    if let Some(arg) = &mut args.varargs {
        get(arg, out)?;
    }
    for (_, arg) in &mut args.kwargs {
        get(arg, out)?;
    }
    if let Some(arg) = &mut args.kwmap {
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
        AssignTargetDesc::Subscript(owner, index) => {
            get(owner, out)?;
            get(index, out)?;
        }
    }
    Ok(())
}

fn getaugtarget(target: &mut AssignTarget, out: &mut State) -> Result<()> {
    let mark = target.mark().clone();
    match target.desc_mut() {
        AssignTargetDesc::Name(name) => {
            if !out.write.contains_key(name) {
                out.write.insert(name.clone(), mark.clone());
            }
            if !out.read.contains_key(name) {
                out.read.insert(name.clone(), mark);
            }
        }
        AssignTargetDesc::List(_) => {
            return Err(rterr!(
                "List patterns cannot be the target of an augmented assignment"
            ));
        }
        AssignTargetDesc::Attr(owner, _attr) => {
            get(owner, out)?;
        }
        AssignTargetDesc::Subscript(owner, index) => {
            get(owner, out)?;
            get(index, out)?;
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

    class_stack: Vec<RcStr>,
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
