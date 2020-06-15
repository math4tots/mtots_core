//! A minimal scripting language
//! For when your scripting language needs a scripting language
//!
//! Meant to be kinda like regular expressions, but for computation
//!
//! Like, if you want to allow the user to modify a parsed value
//! but you don't want to allow the user to pass in an arbitrary closure,
//! and/or you want the computation to be described in a language agnostic
//! way.
//!
//! There's no real error handling, line informatin, etc. etc.
//! It's really just for quickly describing little snippets of computation
//! in a dirty and portable way
//!
//! mini is actually a LISP-2 under the hood, where you can apply an operator
//! 'op' by writing $op(..args),
//! However for convenience, syntactic sugar is allowed (e.g. +, -, *, /, etc)
//! that will be transformed into the $op(..args) form when parsed
//!
mod ast;
mod eval;
mod lexer;
mod parser;
mod scope;
mod val;
use ast::FunctionDisplay;
use ast::Operator;
use lexer::lex;
use lexer::Token;
use val::Function;

pub use ast::Node;
pub use parser::parse;
pub use scope::Scope;
pub use scope::Options;
pub use val::Val;

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn sample() {
        let output = Rc::new(RefCell::new(Vec::<String>::new()));
        let cloned_output = output.clone();
        let scope = Scope::new_root(Options {
            // By default, the $Print operator will call Rust's println!
            // However, this behavior can be overriden, as we do here
            print: Box::new(move |val| {
                cloned_output.borrow_mut().push(format!("{}", val));
                Ok(Val::Nil)
            }),
            ..Options::default()
        });

        let ast = parse(r###"
        nil
        "###).unwrap();
        assert_eq!(ast.eval(&scope), Ok(Val::Nil));

        let ast = parse(r###"
        5 + 7
        "###).unwrap();
        assert_eq!(ast.eval(&scope), Ok(Val::Number(12.0)));

        let ast = parse(r###"
        $Len([1, 2, 3])
        "###).unwrap();
        assert_eq!(ast.eval(&scope), Ok(Val::Number(3.0)));

        output.borrow_mut().clear();
        let ast = parse(r###"
        $Print([1, 2, 3])
        "###).unwrap();
        assert_eq!(ast.eval(&scope), Ok(Val::Nil));
        assert_eq!(output.borrow().clone(), vec![
            "[1, 2, 3]".to_owned(),
        ]);

        output.borrow_mut().clear();
        let ast = parse(r###"
        $Print("Hello world!")
        "###).unwrap();
        assert_eq!(ast.eval(&scope), Ok(Val::Nil));
        assert_eq!(output.borrow().clone(), vec![
            "Hello world!".to_owned(),
        ]);

        output.borrow_mut().clear();
        let ast = parse(r###"
        $Print("Hello world!")
        $Print(15 * 7)
        "###).unwrap();
        assert_eq!(ast.eval(&scope), Ok(Val::Nil));
        assert_eq!(output.borrow().clone(), vec![
            "Hello world!".to_owned(),
            "105".to_owned(),
        ]);
    }
}
