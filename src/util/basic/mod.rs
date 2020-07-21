//! A BASIC dialect

mod ast;
mod code;
mod handler;
mod trans;
mod val;
mod vm;

pub use ast::*;
pub use code::*;
pub use handler::*;
pub use trans::*;
pub use val::*;
pub use vm::*;

pub struct BasicError {
    pub marks: Vec<Mark>,
    pub message: String,
}
