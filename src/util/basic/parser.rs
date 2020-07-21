// use super::lexer::*;
use super::ast::*;
use super::BasicError;
// use super::Mark;
use std::rc::Rc;

pub fn parse(_source: &Rc<Source>) -> Result<File, BasicError> {
    panic!("TODO")
}

// struct Parser<'a> {
//     toks: Vec<(Token<'a>, Mark)>,
//     i: usize,
// }

// impl<'a> Parser<'a> {
//     fn peek(&self) -> &Token<'a> {
//         &self.toks[self.i].0
//     }
//     fn at<P: Into<Pat<'a>>>(&self, p: P) -> bool {
//         let p = p.into();
//         p.matches(self.peek())
//     }
//     fn gettok(&mut self) -> Token<'a> {
//         self.i += 1;
//         std::mem::replace(&mut self.toks[self.i - 1].0, Token::EOF)
//     }
//     fn expect<'b, P: Into<Pat<'b>>>(&mut self, p: P) -> Result<Token<'a>, String> {
//         let p = p.into();
//         if p.matches(self.peek()) {
//             Ok(self.gettok())
//         } else {
//             Err(format!("Expected {:?} but got {:?}", p, self.peek()))
//         }
//     }
//     fn consume<P: Into<Pat<'a>>>(&mut self, p: P) -> bool {
//         if self.at(p) {
//             self.gettok();
//             true
//         } else {
//             false
//         }
//     }
// }

// #[derive(Debug, Clone)]
// enum Pat<'a> {
//     Exact(Token<'a>),
//     Keyword(&'a str),
//     Name,
// }

// impl<'a> Pat<'a> {
//     fn matches<'b>(&self, tok: &Token<'b>) -> bool {
//         match self {
//             Pat::Exact(t) => t == tok,
//             Pat::Keyword(t) => match tok {
//                 Token::Name(name) => t == name,
//                 _ => false,
//             },
//             Pat::Name => match tok {
//                 Token::Name(_) => true,
//                 _ => false,
//             },
//         }
//     }
// }

// impl<'a> From<Token<'a>> for Pat<'a> {
//     fn from(t: Token<'a>) -> Pat<'a> {
//         Pat::Exact(t)
//     }
// }

// impl<'a> From<&'a str> for Pat<'a> {
//     fn from(s: &'a str) -> Pat<'a> {
//         Pat::Keyword(s)
//     }
// }
