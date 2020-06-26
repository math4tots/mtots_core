use crate::Eval;
use crate::Globals;
use crate::RcStr;
use crate::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub(crate) fn run_repl<D: ReplDelegate + ?Sized>(mut globals: Globals, delegate: &mut D) {
    let mut scope = globals.new_repl_scope();
    delegate.accept_scope(&scope);
    'outer: loop {
        if let Some(mut line) = delegate.getline(false) {
            while !globals.is_ready_for_repl(&line) {
                if let Some(next_line) = delegate.getline(true) {
                    line.push_str(&next_line);
                } else {
                    break 'outer;
                }
            }
            match globals.exec_repl(&mut scope, &line) {
                Ok(Value::Nil) => {}
                Ok(value) => match Eval::repr(&mut globals, &value) {
                    Ok(string) => {
                        println!("{}", string);
                    }
                    Err(_) => {
                        assert!(globals.print_if_error());
                    }
                },
                Err(_) => {
                    assert!(globals.print_if_error());
                }
            }
            delegate.accept_scope(&scope);
        } else {
            break;
        }
    }
}

pub trait ReplDelegate {
    fn accept_scope(&mut self, _scope: &ReplScope) {}
    fn getline(&mut self, continuation: bool) -> Option<String>;
}

pub type ReplScope = HashMap<RcStr, Rc<RefCell<Value>>>;
