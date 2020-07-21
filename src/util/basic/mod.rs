//! A BASIC dialect

mod ast;
mod code;
mod handler;
mod trans;
mod val;
mod vm;
mod lexer;
mod parser;

pub use ast::*;
pub use code::*;
pub use handler::*;
pub use trans::*;
pub use val::*;
pub use vm::*;
pub use lexer::*;
pub use parser::*;

#[derive(Debug)]
pub struct BasicError {
    pub marks: Vec<Mark>,
    pub message: String,
}
