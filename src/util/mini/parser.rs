use super::Node;
use super::Val;
use super::Token;
use super::lex;
use super::Operator;
use super::FunctionDisplay;
use std::rc::Rc;

type Prec = i64;
const ADD_PREC: Prec = 60;
const MUL_PREC: Prec = 80;
const UNARY_PREC: Prec = 100;

pub fn parse(s: &str) -> Result<Node, String> {
    let toks = lex(s)?;
    let mut parser = Parser {
        toks,
        i: 0,
    };
    let mut exprs = Vec::new();
    while !parser.at(Token::EOF) {
        exprs.push(expr(&mut parser, 0)?);
    }
    Ok(Node::Block(exprs))
}

struct Parser<'a> {
    toks: Vec<Token<'a>>,
    i: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> &Token<'a> {
        &self.toks[self.i]
    }
    fn at<P: Into<Pat<'a>>>(&self, p: P) -> bool {
        let p = p.into();
        p.matches(self.peek())
    }
    fn gettok(&mut self) -> Token<'a> {
        self.i += 1;
        std::mem::replace(&mut self.toks[self.i - 1], Token::EOF)
    }
    fn expect<'b, P: Into<Pat<'b>>>(&mut self, p: P) -> Result<Token<'a>, String> {
        let p = p.into();
        if p.matches(self.peek()) {
            Ok(self.gettok())
        } else {
            Err(format!("Expected {:?} but got {:?}", p, self.peek()))
        }
    }
    fn consume<P: Into<Pat<'a>>>(&mut self, p: P) -> bool {
        if self.at(p) {
            self.gettok();
            true
        } else {
            false
        }
    }
}

fn consume_delim(p: &mut Parser) {
    while p.at(Token::Newline) || p.at(Token::Semicolon) {
        p.gettok();
    }
}

fn delim(p: &mut Parser) -> Result<(), String> {
    match p.peek() {
        Token::RBrace | Token::EOF | Token::Semicolon | Token::Newline => (),
        t => return Err(format!("Expected dlimiter but got {:?}", t)),
    }
    consume_delim(p);
    Ok(())
}

fn atom(p: &mut Parser) -> Result<Node, String> {
    match p.gettok() {
        Token::LParen => {
            let e = expr(p, 0)?;
            p.expect(Token::RParen)?;
            Ok(e)
        }
        Token::Number(x) => {
            Ok(Node::Constant(Val::Number(x)))
        }
        Token::String(x) => {
            Ok(Node::Constant(Val::String(x.into())))
        }
        Token::RawString(x) => {
            Ok(Node::Constant(Val::String(x.to_owned().into())))
        }
        Token::Plus => {
            let e = expr(p, UNARY_PREC)?;
            Ok(Node::Operation(Operator::Pos, vec![e]))
        }
        Token::Minus => {
            let e = expr(p, UNARY_PREC)?;
            Ok(Node::Operation(Operator::Neg, vec![e]))
        }
        Token::LBrace => block(p),
        Token::LBracket => {
            let mut exprs = Vec::new();
            while !p.at(Token::RBracket) {
                exprs.push(expr(p, 0)?);
                if !p.consume(Token::Comma) {
                    p.expect(Token::RBracket)?;
                    break;
                }
            }
            Ok(Node::ListDisplay(exprs))
        }
        Token::Bar => {
            let mut params = Vec::new();
            while !p.consume(Token::Bar) {
                params.push(p.expect(Pat::Name)?.name().unwrap().to_owned().into());
                if !p.consume(Token::Comma) {
                    p.expect(Token::Bar)?;
                    break;
                }
            }
            let body = expr(p, 0)?;
            Ok(Node::FunctionDisplay(Rc::new(FunctionDisplay::new(params, body))))
        }
        Token::Name(name) => {
            match name {
                "nil" => Ok(Node::Constant(Val::Nil)),
                "while" => {
                    let cond = expr(p, 0)?;
                    p.expect(Token::LBrace)?;
                    let body = block(p)?;
                    Ok(Node::While(cond.into(), body.into()))
                }
                "if" | "else" | "elif" | "true" | "false" | "class" | "struct" => {
                    Err(format!("{:?} is a reserved name", name))
                }
                _ => {
                    if p.consume(Token::Eq) {
                        let e = expr(p, 0)?.into();
                        Ok(Node::SetVar(name.to_owned().into(), e))
                    } else {
                        Ok(Node::GetVar(name.to_owned().into()))
                    }
                }
            }
        }
        Token::Dollar => {
            let optok = p.expect(Pat::Name)?;
            let opname = optok.name().unwrap();
            let op = match Operator::from_str(opname) {
                Some(op) => op,
                None => return Err(format!("Unknown operator {:?}", opname)),
            };
            p.expect(Token::LParen)?;
            let mut exprs = Vec::new();
            while !p.consume(Token::RParen) {
                exprs.push(expr(p, 0)?);
                if !p.consume(Token::Comma) {
                    p.expect(Token::RParen)?;
                    break;
                }
            }
            Ok(Node::Operation(op, exprs))
        }
        tok => Err(format!("Expected expression but got {:?}", tok)),
    }
}

fn block(p: &mut Parser) -> Result<Node, String> {
    let mut exprs = Vec::new();
    consume_delim(p);
    while !p.at(Token::RBrace) {
        exprs.push(expr(p, 0)?);
        delim(p)?;
    }
    Ok(Node::Block(exprs))
}

/// Parses an expression using all operations with at least 'prec' level
/// of precedence or higher
fn expr(p: &mut Parser, prec: Prec) -> Result<Node, String> {
    let mut e = atom(p)?;
    while precof(p.peek()) >= prec {
        e = infix(p, e)?;
    }
    Ok(e)
}

/// like expr, but excludes 'prec' itself
/// For left-associative binary operators
fn expre(p: &mut Parser, prec: Prec) -> Result<Node, String> {
    expr(p, prec - 1)
}

fn precof<'a>(tok: &Token<'a>) -> Prec {
    match tok {
        Token::Minus | Token::Plus => ADD_PREC,
        Token::Star | Token::Slash | Token::Slash2 | Token::Rem => MUL_PREC,
        _ => -1,
    }
}

fn infix(p: &mut Parser, lhs: Node) -> Result<Node, String> {
    let tok = p.gettok();
    match &tok {
        // normal left-associative binary operators
        Token::Plus | Token::Minus | Token::Star | Token::Slash | Token::Slash2 | Token::Rem => {
            let op = match &tok {
                Token::Plus => Operator::Add,
                Token::Minus => Operator::Sub,
                Token::Star => Operator::Mul,
                Token::Slash => Operator::Div,
                Token::Slash2 => Operator::TruncDiv,
                Token::Rem => Operator::Rem,
                _ => panic!("{:?}", tok),
            };
            let prec = precof(&tok);
            let rhs = expre(p, prec)?;
            Ok(Node::Operation(op, vec![lhs, rhs]))
        }
        tok => Err(format!("Expected infix operator but got {:?}", tok)),
    }
}


#[derive(Debug, Clone)]
enum Pat<'a> {
    Exact(Token<'a>),
    Keyword(&'a str),
    Name,
}

impl<'a> Pat<'a> {
    fn matches<'b>(&self, tok: &Token<'b>) -> bool {
        match self {
            Pat::Exact(t) => t == tok,
            Pat::Keyword(t) => match tok {
                Token::Name(name) => t == name,
                _ => false,
            }
            Pat::Name => match tok {
                Token::Name(_) => true,
                _ => false,
            }
        }
    }
}

impl<'a> From<Token<'a>> for Pat<'a> {
    fn from(t: Token<'a>) -> Pat<'a> {
        Pat::Exact(t)
    }
}

impl<'a> From<&'a str> for Pat<'a> {
    fn from(s: &'a str) -> Pat<'a> {
        Pat::Keyword(s)
    }
}
