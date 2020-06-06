use crate::ArgumentList;
use crate::Binop;
use crate::Expression;
use crate::ExpressionData;
use crate::ExpressionKind;
use crate::Punctuator;
use crate::RcStr;
use crate::Symbol;
use crate::SymbolRegistryHandle;
use crate::Token;
use crate::TokenKind;
use crate::Unop;

const PREC_STEP: i32 = 10;

#[derive(Debug)]
pub struct ParseError {
    offset: usize,
    lineno: usize,
    kind: ParseErrorKind,
}

#[derive(Debug)]
pub enum ParseErrorKind {
    InvalidToken {
        expected: TokenKind,
        but_got: TokenKind,
    },
    ExpectedExpression {
        but_got: TokenKind,
    },
    ExpectedParameter {
        but_got: TokenKind,
    },
    ExpectedDelimiter {
        but_got: TokenKind,
    },
    ExpectedString {
        but_got: TokenKind,
    },
    InvalidParameterOrder {
        parameter_kind: ParameterKind,
        cannot_come_after: ParameterKind,
    },
    IllegalDuplicateParameterKind(ParameterKind),
    MultipleVariadicArgs,
    MultipleKeywordTableArgs,
    IllegalArgumentOrder,
    UnrecognizedEscapeSequence(String),
    InvalidEscape(String),
    EscapeAtEndOfString,
    ExpectedPotentiallyMutableExpression(ExpressionKind),
    ExpectedClassMember {
        but_got: ExpressionKind,
    },
    FieldListInTrait,
}

impl ParseError {
    pub fn move_(self) -> (usize, usize, ParseErrorKind) {
        (self.offset, self.lineno, self.kind)
    }

    pub fn lineno(&self) -> usize {
        self.lineno
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParameterKind {
    Required,
    Optional,
    Variadic,
    Keywords,
}

impl ParameterKind {
    /// whether the given parameter kind is allowed multiple times
    /// in a parameter list
    pub fn multiple_allowed(&self) -> bool {
        match self {
            ParameterKind::Required | ParameterKind::Optional => true,
            _ => false,
        }
    }
}

pub struct Parser {
    prectable: Vec<Prec>,
    prefix_table: Vec<Option<for<'a> fn(&mut ParserState<'a>) -> Result<Expression, ParseError>>>,
    infix_table:
        Vec<Option<fn(&mut ParserState, Expression, Prec) -> Result<Expression, ParseError>>>,
    symbol_registry: SymbolRegistryHandle,
}

impl Parser {
    pub(crate) fn new(symbol_registry: SymbolRegistryHandle) -> Parser {
        let (infix_table, prectable) = geninfix();
        Parser {
            prectable,
            prefix_table: genprefix(),
            infix_table,
            symbol_registry,
        }
    }

    pub fn parse_tokens<'a>(
        &self,
        tokens: Vec<Token<'a>>,
        posinfo: Vec<(usize, usize)>,
    ) -> Result<Expression, ParseError> {
        let mut state = ParserState {
            i: 0,
            tokens,
            posinfo,
            prectable: &self.prectable,
            prefix_table: &self.prefix_table,
            infix_table: &self.infix_table,
            symbol_registry: self.symbol_registry.clone(),
        };
        state.parse()
    }
}

type Prec = i32;

struct ParserState<'a> {
    i: usize,
    tokens: Vec<Token<'a>>,
    posinfo: Vec<(usize, usize)>,
    prectable: &'a Vec<Prec>,
    prefix_table: &'a Vec<Option<fn(&mut ParserState) -> Result<Expression, ParseError>>>,
    infix_table:
        &'a Vec<Option<fn(&mut ParserState, Expression, Prec) -> Result<Expression, ParseError>>>,
    symbol_registry: SymbolRegistryHandle,
}

impl<'a> ParserState<'a> {
    fn peek(&self) -> Token<'a> {
        self.tokens[self.i]
    }

    fn peek1(&self) -> Option<Token<'a>> {
        self.tokens.get(self.i + 1).cloned()
    }

    fn pos(&self) -> (usize, usize) {
        self.posinfo[self.i]
    }

    fn gettok(&mut self) -> Token<'a> {
        let token = self.peek();
        self.i += 1;
        token
    }

    fn consume(&mut self, kind: TokenKind) -> bool {
        if self.peek().kind() == kind {
            self.gettok();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, expected: TokenKind) -> Result<Token<'a>, ParseError> {
        if self.peek().kind() == expected {
            Ok(self.gettok())
        } else {
            let (offset, lineno) = self.pos();
            Err(ParseError {
                offset,
                lineno,
                kind: ParseErrorKind::InvalidToken {
                    expected,
                    but_got: self.peek().kind(),
                },
            })
        }
    }

    fn expect_name(&mut self) -> Result<&'a str, ParseError> {
        Ok(self.expect(TokenKind::Name)?.name().unwrap())
    }

    fn expect_symbol(&mut self) -> Result<&'a str, ParseError> {
        Ok(self.expect(TokenKind::Symbol)?.symbol().unwrap())
    }

    fn consume_docstring(&mut self) -> Option<RcStr> {
        if self.at_string() {
            Some(self.expect_string().unwrap())
        } else {
            None
        }
    }

    fn consume_fields(&mut self) -> Result<Option<Vec<Symbol>>, ParseError> {
        if self.consume(TokenKind::Punctuator(Punctuator::LBracket)) {
            let mut fields = Vec::new();
            while !self.consume(TokenKind::Punctuator(Punctuator::RBracket)) {
                fields.push(self.expect_name_as_symbol()?);
                if !self.consume(TokenKind::Punctuator(Punctuator::Comma)) {
                    self.expect(TokenKind::Punctuator(Punctuator::RBracket))?;
                    break;
                }
            }
            self.skip_delim();
            Ok(Some(fields))
        } else {
            Ok(None)
        }
    }

    fn expect_name_as_symbol(&mut self) -> Result<Symbol, ParseError> {
        let rcstr: RcStr = self.expect_name()?.into();
        Ok(self.symbol_registry.intern_rcstr(&rcstr))
    }

    fn expect_symbol_as_symbol(&mut self) -> Result<Symbol, ParseError> {
        let rcstr: RcStr = self.expect_symbol()?.into();
        Ok(self.symbol_registry.intern_rcstr(&rcstr))
    }

    fn expect_delim(&mut self) -> Result<(), ParseError> {
        if self.at_delim() {
            self.skip_delim();
            Ok(())
        } else {
            let (offset, lineno) = self.pos();
            Err(ParseError {
                offset,
                lineno,
                kind: ParseErrorKind::ExpectedDelimiter {
                    but_got: self.peek().kind(),
                },
            })
        }
    }

    fn at_delim(&self) -> bool {
        if let Token::Newline(_)
        | Token::Punctuator(Punctuator::Semicolon)
        | Token::Punctuator(Punctuator::RBrace)
        | Token::EOF = self.peek()
        {
            true
        } else {
            false
        }
    }

    fn skip_delim(&mut self) {
        while let Token::Newline(_) | Token::Punctuator(Punctuator::Semicolon) = self.peek() {
            self.gettok();
        }
    }

    fn at_string(&self) -> bool {
        if let Token::NormalString(_) | Token::RawString(_) | Token::LineString(_) = self.peek() {
            true
        } else {
            false
        }
    }

    fn expect_string(&mut self) -> Result<RcStr, ParseError> {
        match self.peek() {
            Token::NormalString(s) => {
                self.gettok();
                let raw_string = s;
                match interpret_string(raw_string) {
                    Ok(value) => Ok(value.into()),
                    Err(error) => {
                        let InterpretationError { offset, kind } = error;
                        let (start_offset, start_lineno) = self.pos();
                        Err(ParseError {
                            offset: start_offset + offset,
                            lineno: start_lineno,
                            kind: match kind {
                                InterpretationErrorKind::EscapeAtEndOfString => {
                                    ParseErrorKind::EscapeAtEndOfString
                                }
                                InterpretationErrorKind::InvalidEscape(s) => {
                                    ParseErrorKind::InvalidEscape(s)
                                }
                            },
                        })
                    }
                }
            }
            Token::RawString(s) => {
                self.gettok();
                Ok(s.into())
            }
            Token::LineString(_) => {
                let mut ret = String::new();
                while let Token::LineString(s) = self.peek() {
                    ret.push_str(s);
                    ret.push('\n');
                    self.gettok();

                    // If the next line-string is separated by a single newline,
                    // allow merging with the next one
                    if self.peek() == Token::Newline(1)
                        && self
                            .peek1()
                            .map(|t| t.line_string().is_some())
                            .unwrap_or(false)
                    {
                        self.consume(TokenKind::Newline);
                    }
                }
                Ok(ret.into())
            }
            _ => {
                let (offset, lineno) = self.pos();
                Err(ParseError {
                    offset,
                    lineno,
                    kind: ParseErrorKind::ExpectedString {
                        but_got: self.peek().kind(),
                    },
                })
            }
        }
    }

    fn parse(&mut self) -> Result<Expression, ParseError> {
        let mut exprs = Vec::new();
        self.skip_delim();
        while self.peek().kind() != TokenKind::EOF {
            exprs.push(self.stmt()?);
            self.expect_delim()?;
        }
        Ok(Expression::new(0, 1, ExpressionData::Block(exprs)))
    }

    fn prec(&self, kind: TokenKind) -> Prec {
        self.prectable[kind.id()]
    }

    fn cprec(&self) -> Prec {
        self.prec(self.peek().kind())
    }

    fn block_ex(&mut self, nil_appended: bool) -> Result<(Option<RcStr>, Expression), ParseError> {
        let (offset, lineno) = self.pos();
        self.expect(TokenKind::Punctuator(Punctuator::LBrace))?;
        let mut exprs = Vec::new();
        self.skip_delim();
        while !self.consume(TokenKind::Punctuator(Punctuator::RBrace)) {
            exprs.push(self.stmt()?);
            self.expect_delim()?;
        }
        let docstr = if let Some(ExpressionData::String(s)) = exprs.get(0).map(|e| e.data()) {
            Some(s.clone())
        } else {
            None
        };
        if nil_appended {
            exprs.push(Expression::new(offset, lineno, ExpressionData::Nil));
        }
        Ok((
            docstr,
            Expression::new(offset, lineno, ExpressionData::Block(exprs)),
        ))
    }

    fn block(&mut self) -> Result<Expression, ParseError> {
        Ok(self.block_ex(false)?.1)
    }

    fn block_with_doc(&mut self) -> Result<(Option<RcStr>, Expression), ParseError> {
        self.block_ex(false)
    }

    fn nil_appended_block_with_doc(&mut self) -> Result<(Option<RcStr>, Expression), ParseError> {
        self.block_ex(true)
    }

    /// parse an expression with the given precedence
    fn expr(&mut self, prec: Prec) -> Result<Expression, ParseError> {
        let mut expr = self.prefix()?;
        while self.cprec() > prec {
            expr = self.infix(expr)?;
        }
        Ok(expr)
    }

    /// parse a statement
    /// a statement is basically an expression, except that named functions
    /// will automatically be assigned to a variable of the same name
    fn stmt(&mut self) -> Result<Expression, ParseError> {
        let expr = self.expr(0)?;

        // If we see an assignment followed by a '#' string on the next line,
        // we assume that the string is meant to be a doc for the assignment
        if let Some(name) = Self::get_assign_name(&expr) {
            if let Some(doc) = self.followup_doc()? {
                let offset = expr.offset();
                let lineno = expr.lineno();
                return Ok(Expression::new(
                    offset,
                    lineno,
                    ExpressionData::AssignWithDoc(expr.into(), name, doc),
                ));
            }
        }

        Ok(match expr.data() {
            ExpressionData::FunctionDisplay(_, Some(name), ..) => assign_name(name.clone(), expr),
            ExpressionData::ClassDisplay(_, name, ..) => assign_name(name.clone(), expr),
            ExpressionData::ExceptionKindDisplay(name, ..) => assign_name(name.clone(), expr),
            _ => expr,
        })
    }

    fn get_assign_name(expr: &Expression) -> Option<RcStr> {
        if let ExpressionData::Assign(target, _) = expr.data() {
            if let ExpressionData::Name(name) = target.data() {
                return Some(name.clone());
            }
        }
        None
    }

    fn followup_doc(&mut self) -> Result<Option<RcStr>, ParseError> {
        if self.peek() == Token::Newline(1)
            && self
                .peek1()
                .map(|t| t.line_string().is_some())
                .unwrap_or(false)
        {
            self.expect(TokenKind::Newline)?;
            Ok(Some(self.expect_string()?))
        } else {
            Ok(None)
        }
    }

    fn prefix(&mut self) -> Result<Expression, ParseError> {
        let key = self.peek().kind().id();
        if let Some(f) = self.prefix_table[key] {
            f(self)
        } else {
            let (offset, lineno) = self.pos();
            Err(ParseError {
                offset,
                lineno,
                kind: ParseErrorKind::ExpectedExpression {
                    but_got: self.peek().kind(),
                },
            })
        }
    }

    fn infix(&mut self, expr: Expression) -> Result<Expression, ParseError> {
        let key = self.peek().kind().id();
        let prec = self.cprec();
        self.infix_table[key].unwrap()(self, expr, prec)
    }

    fn params(
        &mut self,
    ) -> Result<
        (
            Vec<RcStr>,
            Vec<(RcStr, Expression)>,
            Option<RcStr>,
            Option<RcStr>,
        ),
        ParseError,
    > {
        self.expect(TokenKind::Punctuator(Punctuator::LParen))?;
        let mut req = Vec::new(); // required params
        let mut opt = Vec::new(); // optional params
        let mut variadic = None;
        let mut keywords = None;
        let mut last_kind = ParameterKind::Required;
        while !self.consume(TokenKind::Punctuator(Punctuator::RParen)) {
            let (offset, lineno) = self.pos();
            let kind = match self.gettok() {
                Token::Name(name) => {
                    if self.consume(TokenKind::Punctuator(Punctuator::Eq)) {
                        // optional parameter
                        let expr = self.expr(0)?;
                        opt.push((name.into(), expr));
                        ParameterKind::Optional
                    } else {
                        // required parameter
                        req.push(name.into());
                        ParameterKind::Required
                    }
                }
                Token::Punctuator(Punctuator::Star) => {
                    // variadic parameter
                    variadic = Some(self.expect_name()?.into());
                    ParameterKind::Variadic
                }
                Token::Punctuator(Punctuator::Star2) => {
                    // keyword table parameter
                    keywords = Some(self.expect_name()?.into());
                    ParameterKind::Keywords
                }
                token => {
                    return Err(ParseError {
                        offset,
                        lineno,
                        kind: ParseErrorKind::ExpectedParameter {
                            but_got: token.kind(),
                        },
                    })
                }
            };
            if last_kind > kind {
                return Err(ParseError {
                    offset,
                    lineno,
                    kind: ParseErrorKind::InvalidParameterOrder {
                        parameter_kind: kind,
                        cannot_come_after: last_kind,
                    },
                });
            }
            if !kind.multiple_allowed() && last_kind == kind {
                return Err(ParseError {
                    offset,
                    lineno,
                    kind: ParseErrorKind::IllegalDuplicateParameterKind(kind),
                });
            }
            last_kind = kind;
            if !self.consume(TokenKind::Punctuator(Punctuator::Comma)) {
                self.expect(TokenKind::Punctuator(Punctuator::RParen))?;
                break;
            }
        }
        Ok((req, opt, variadic, keywords))
    }

    fn args(&mut self) -> Result<ArgumentList, ParseError> {
        self.expect(TokenKind::Punctuator(Punctuator::LParen))?;
        let mut pos = Vec::new();
        let mut key = Vec::new();
        let mut variadic = None;
        let mut kwtable = None;
        while !self.consume(TokenKind::Punctuator(Punctuator::RParen)) {
            let (offset, lineno) = self.pos();
            if self.consume(TokenKind::Punctuator(Punctuator::Star)) {
                // kwtables have to come after vararg arguments
                if kwtable.is_some() {
                    return Err(ParseError {
                        offset,
                        lineno,
                        kind: ParseErrorKind::IllegalArgumentOrder,
                    });
                }
                if variadic.is_some() {
                    return Err(ParseError {
                        offset,
                        lineno,
                        kind: ParseErrorKind::MultipleVariadicArgs,
                    });
                }
                variadic = Some(self.expr(0)?);
            } else if self.consume(TokenKind::Punctuator(Punctuator::Star2)) {
                if kwtable.is_some() {
                    return Err(ParseError {
                        offset,
                        lineno,
                        kind: ParseErrorKind::MultipleKeywordTableArgs,
                    });
                }
                kwtable = Some(self.expr(0)?);
            } else if self.peek().kind() == TokenKind::Name
                && self.tokens.get(self.i + 1).map(|t| t.kind())
                    == Some(TokenKind::Punctuator(Punctuator::Eq))
            {
                // variadic and kwtables have to come after vararg arguments
                if variadic.is_some() || kwtable.is_some() {
                    return Err(ParseError {
                        offset,
                        lineno,
                        kind: ParseErrorKind::IllegalArgumentOrder,
                    });
                }
                let name = self.expect_name()?;
                self.expect(TokenKind::Punctuator(Punctuator::Eq))?;
                let expr = self.expr(0)?;
                key.push((name.into(), expr));
            } else {
                // keyword, variadic and kwtables have to come after vararg arguments
                if !key.is_empty() || variadic.is_some() || kwtable.is_some() {
                    return Err(ParseError {
                        offset,
                        lineno,
                        kind: ParseErrorKind::IllegalArgumentOrder,
                    });
                }
                pos.push(self.expr(0)?);
            }

            if !self.consume(TokenKind::Punctuator(Punctuator::Comma)) {
                self.expect(TokenKind::Punctuator(Punctuator::RParen))?;
                break;
            }
        }
        Ok(ArgumentList::new(pos, key, variadic, kwtable))
    }
}

/// generate the prefix table
/// a prefix table maps
///     token kinds to a <parsing callback> that can parse an expression
///     that starts with the given token kind
fn genprefix() -> Vec<Option<fn(&mut ParserState) -> Result<Expression, ParseError>>> {
    let entries: Vec<(
        &[&'static str],
        fn(&mut ParserState) -> Result<Expression, ParseError>,
    )> = vec![
        (&["nil"], |state: &mut ParserState| {
            mk1tokexpr(state, ExpressionData::Nil)
        }),
        (&["true"], |state: &mut ParserState| {
            mk1tokexpr(state, ExpressionData::Bool(true))
        }),
        (&["false"], |state: &mut ParserState| {
            mk1tokexpr(state, ExpressionData::Bool(false))
        }),
        (&["Int"], |state: &mut ParserState| {
            let value = state.peek().int().unwrap();
            mk1tokexpr(state, ExpressionData::Int(value))
        }),
        (&["Float"], |state: &mut ParserState| {
            let value = state.peek().float().unwrap();
            mk1tokexpr(state, ExpressionData::Float(value))
        }),
        (&["Symbol"], |state: &mut ParserState| {
            let (offset, lineno) = state.pos();
            let symbol = state.expect_symbol_as_symbol()?;
            Ok(Expression::new(
                offset,
                lineno,
                ExpressionData::Symbol(symbol),
            ))
        }),
        (
            &["NormalString", "RawString", "LineString"],
            |state: &mut ParserState| {
                let (offset, lineno) = state.pos();
                let s = state.expect_string()?;
                Ok(Expression::new(
                    offset,
                    lineno,
                    ExpressionData::String(s.into()),
                ))
            },
        ),
        (&["Name"], |state: &mut ParserState| {
            let name = state.peek().name().unwrap();
            mk1tokexpr(state, ExpressionData::Name(name.into()))
        }),
        (&["{"], |state: &mut ParserState| state.block()),
        (&["("], |state: &mut ParserState| {
            let (offset, lineno) = state.pos();
            state.gettok();
            let expr = state.expr(0)?;
            state.expect(TokenKind::Punctuator(Punctuator::RParen))?;
            Ok(Expression::new(
                offset,
                lineno,
                ExpressionData::Parentheses(expr.into()),
            ))
        }),
        (&["["], |state: &mut ParserState| {
            let (offset, lineno) = state.pos();
            state.gettok();
            // For the special case of the empty map, we have to do an extra token of
            // lookahead, because ':' is permitted at the beginning of an expression
            // (specifically for symbol literals)
            if state.peek().kind() == TokenKind::Punctuator(Punctuator::Colon)
                && state.tokens.get(state.i + 1).map(|t| t.kind())
                    == Some(TokenKind::Punctuator(Punctuator::RBracket))
            {
                // Empty map
                state.expect(TokenKind::Punctuator(Punctuator::Colon))?;
                state.expect(TokenKind::Punctuator(Punctuator::RBracket))?;
                Ok(Expression::new(
                    offset,
                    lineno,
                    ExpressionData::MapDisplay(vec![]),
                ))
            } else if state.consume(TokenKind::Punctuator(Punctuator::RBracket)) {
                // Empty list
                Ok(Expression::new(
                    offset,
                    lineno,
                    ExpressionData::ListDisplay(vec![]),
                ))
            } else {
                let first = state.expr(0)?;
                if state.consume(TokenKind::Punctuator(Punctuator::Colon)) {
                    // map
                    let first_val = state.expr(0)?;
                    let mut pairs = vec![(first, first_val)];
                    loop {
                        if !state.consume(TokenKind::Punctuator(Punctuator::Comma)) {
                            state.expect(TokenKind::Punctuator(Punctuator::RBracket))?;
                            break;
                        }
                        if state.consume(TokenKind::Punctuator(Punctuator::RBracket)) {
                            break;
                        }
                        let key = state.expr(0)?;
                        state.expect(TokenKind::Punctuator(Punctuator::Colon))?;
                        let val = state.expr(0)?;
                        pairs.push((key, val));
                    }
                    Ok(Expression::new(
                        offset,
                        lineno,
                        ExpressionData::MapDisplay(pairs),
                    ))
                } else {
                    // list
                    let mut exprs = vec![first];
                    loop {
                        if !state.consume(TokenKind::Punctuator(Punctuator::Comma)) {
                            state.expect(TokenKind::Punctuator(Punctuator::RBracket))?;
                            break;
                        }
                        if state.consume(TokenKind::Punctuator(Punctuator::RBracket)) {
                            break;
                        }
                        exprs.push(state.expr(0)?);
                    }
                    Ok(Expression::new(
                        offset,
                        lineno,
                        ExpressionData::ListDisplay(exprs),
                    ))
                }
            }
        }),
        (&["if"], |state: &mut ParserState| {
            let (offset, lineno) = state.pos();
            state.gettok();
            let cond = state.expr(0)?;
            let body = state.block()?;
            let mut pairs = vec![(cond, body)];
            while state.consume(TokenKind::Punctuator(Punctuator::Elif)) {
                let cond = state.expr(0)?;
                let body = state.block()?;
                pairs.push((cond, body));
            }
            let other = if state.consume(TokenKind::Punctuator(Punctuator::Else)) {
                Some(state.block()?.into())
            } else {
                None
            };
            Ok(Expression::new(
                offset,
                lineno,
                ExpressionData::If(pairs, other),
            ))
        }),
        (&["for"], |state: &mut ParserState| {
            let (offset, lineno) = state.pos();
            state.gettok();
            let target = state.expr(0)?.into();
            state.expect(TokenKind::Punctuator(Punctuator::In))?;
            let iterable = state.expr(0)?.into();
            let body = state.block()?.into();
            Ok(Expression::new(
                offset,
                lineno,
                ExpressionData::For(target, iterable, body),
            ))
        }),
        (&["while"], |state: &mut ParserState| {
            let (offset, lineno) = state.pos();
            state.gettok();
            let cond = state.expr(0)?.into();
            let body = state.block()?.into();
            Ok(Expression::new(
                offset,
                lineno,
                ExpressionData::While(cond, body),
            ))
        }),
        (&["del"], |state: &mut ParserState| {
            let (offset, lineno) = state.pos();
            state.gettok();
            let varname = state.expect_name()?;
            Ok(Expression::new(
                offset,
                lineno,
                ExpressionData::Del(varname.into()),
            ))
        }),
        (&["nonlocal"], |state: &mut ParserState| {
            let (offset, lineno) = state.pos();
            state.gettok();
            let mut names = Vec::new();
            while state.peek().kind() == TokenKind::Name {
                names.push(state.expect_name()?.into());
                if !state.consume(TokenKind::Punctuator(Punctuator::Comma)) {
                    break;
                }
            }
            Ok(Expression::new(
                offset,
                lineno,
                ExpressionData::Nonlocal(names),
            ))
        }),
        (&["yield"], |state: &mut ParserState| {
            let (offset, lineno) = state.pos();
            state.gettok();
            let expr = state.expr(0)?;
            Ok(Expression::new(
                offset,
                lineno,
                ExpressionData::Yield(expr.into()),
            ))
        }),
        (&["return"], |state: &mut ParserState| {
            let (offset, lineno) = state.pos();
            state.gettok();
            let expr = if state.at_delim() {
                None
            } else {
                Some(state.expr(0)?.into())
            };
            Ok(Expression::new(
                offset,
                lineno,
                ExpressionData::Return(expr),
            ))
        }),
        (&["def"], |state: &mut ParserState| {
            let (offset, lineno) = state.pos();
            state.gettok();

            // 'def break' is special syntax to indicate a breakpoint
            if state.consume(TokenKind::Punctuator(Punctuator::Break)) {
                return Ok(Expression::new(offset, lineno, ExpressionData::BreakPoint));
            }

            // otherwise we're dealing with a function definition
            let is_generator = state.consume(TokenKind::Punctuator(Punctuator::Star));
            let name = if state.peek().kind() == TokenKind::Name {
                Some(state.expect_name()?.into())
            } else {
                None
            };
            let (req, opt, var, kw) =
                if state.peek().kind() == TokenKind::Punctuator(Punctuator::LParen) {
                    state.params()?
                } else {
                    (vec![], vec![], None, None)
                };
            let (doc, body) = if state.consume(TokenKind::Punctuator(Punctuator::Eq)) {
                if state.peek() == Token::Punctuator(Punctuator::LBrace) {
                    state.block_with_doc()?
                } else {
                    (None, state.expr(0)?)
                }
            } else {
                state.nil_appended_block_with_doc()?
            };
            Ok(Expression::new(
                offset,
                lineno,
                ExpressionData::FunctionDisplay(
                    is_generator,
                    name,
                    req,
                    opt,
                    var,
                    kw,
                    doc,
                    body.into(),
                ),
            ))
        }),
        (&["except"], |state: &mut ParserState| {
            let (offset, lineno) = state.pos();
            state.gettok();
            let short_name = state.expect_name()?.into();
            let base = if state.consume(TokenKind::Punctuator(Punctuator::LParen)) {
                let base = state.expr(0)?;
                state.expect(TokenKind::Punctuator(Punctuator::RParen))?;
                Some(base.into())
            } else {
                None
            };
            let (docstring, fields, template) = {
                state.expect(TokenKind::Punctuator(Punctuator::LBrace))?;
                state.skip_delim();
                let docstring = state.consume_docstring();

                let fields = state.consume_fields()?;

                state.expect(TokenKind::Punctuator(Punctuator::Def))?;
                let template = state.expr(0)?.into();
                state.expect_delim()?;
                state.expect(TokenKind::Punctuator(Punctuator::RBrace))?;

                (docstring, fields, template)
            };
            Ok(Expression::new(
                offset,
                lineno,
                ExpressionData::ExceptionKindDisplay(
                    short_name,
                    base,
                    docstring.into(),
                    fields,
                    template,
                ),
            ))
        }),
        (&["class", "trait"], |state: &mut ParserState| {
            let (offset, lineno) = state.pos();
            let is_trait = state.peek().kind() == TokenKind::Punctuator(Punctuator::Trait);
            state.gettok();
            let short_name = state.expect_name()?.into();
            let bases = {
                let mut bases = Vec::new();
                if state.consume(TokenKind::Punctuator(Punctuator::LParen)) {
                    while !state.consume(TokenKind::Punctuator(Punctuator::RParen)) {
                        bases.push(state.expr(0)?);
                        if !state.consume(TokenKind::Punctuator(Punctuator::Comma)) {
                            state.expect(TokenKind::Punctuator(Punctuator::RParen))?;
                            break;
                        }
                    }
                }
                bases
            };
            let (docstring, fields, methods, static_methods) = {
                let mut docstring = None;
                let mut fields = None;
                let mut methods = Vec::new();
                let mut static_methods = Vec::new();
                if state.consume(TokenKind::Punctuator(Punctuator::LBrace)) {
                    state.skip_delim();

                    docstring = state.consume_docstring();

                    state.skip_delim();

                    let (offset, lineno) = state.pos();
                    fields = match state.consume_fields()? {
                        Some(fields) => {
                            if is_trait {
                                return Err(ParseError {
                                    offset,
                                    lineno,
                                    kind: ParseErrorKind::FieldListInTrait,
                                });
                            }
                            Some(fields)
                        }
                        None => None,
                    };

                    while !state.consume(TokenKind::Punctuator(Punctuator::RBrace)) {
                        let out = if state.consume(TokenKind::Punctuator(Punctuator::Static)) {
                            &mut static_methods
                        } else {
                            &mut methods
                        };
                        let stmt = state.stmt()?;
                        let offset = stmt.offset();
                        let lineno = stmt.lineno();
                        let name = match get_simple_assignment_name(&stmt) {
                            Some(name) => name.clone(),
                            None => {
                                return Err(ParseError {
                                    offset,
                                    lineno,
                                    kind: ParseErrorKind::ExpectedClassMember {
                                        but_got: stmt.kind(),
                                    },
                                });
                            }
                        };
                        out.push((name, stmt));
                        state.expect_delim()?;
                    }
                }
                (docstring, fields, methods, static_methods)
            };
            Ok(Expression::new(
                offset,
                lineno,
                ExpressionData::ClassDisplay(
                    is_trait,
                    short_name,
                    bases,
                    docstring,
                    fields,
                    methods,
                    static_methods,
                ),
            ))
        }),
        (&["import"], |state: &mut ParserState| {
            let (offset, lineno) = state.pos();
            state.gettok();
            let mut name = String::new();
            while state.consume(TokenKind::Punctuator(Punctuator::Dot)) {
                name.push('.');
            }
            let mut last_part = state.expect_name()?;
            name.push_str(last_part);
            while state.consume(TokenKind::Punctuator(Punctuator::Dot)) {
                name.push('.');
                last_part = state.expect_name()?;
                name.push_str(last_part);
            }
            let field = if state.consume(TokenKind::Punctuator(Punctuator::Scope)) {
                last_part = state.expect_name()?;
                Some(last_part)
            } else {
                None
            };
            let alias = if state.consume(TokenKind::Punctuator(Punctuator::As)) {
                state.expect_name()?
            } else {
                last_part
            };
            let raw_import = Expression::new(offset, lineno, ExpressionData::Import(name.into()));
            let field_applied = match field {
                Some(field) => Expression::new(
                    offset,
                    lineno,
                    ExpressionData::StaticAttribute(raw_import.into(), field.into()),
                ),
                None => raw_import,
            };
            Ok(assign_name(alias.into(), field_applied))
        }),
        (&["-", "+", "!"], |state| {
            let op = match state.peek().kind() {
                TokenKind::Punctuator(Punctuator::Plus) => Unop::Pos,
                TokenKind::Punctuator(Punctuator::Minus) => Unop::Neg,
                TokenKind::Punctuator(Punctuator::Exclamation) => Unop::Not,
                _ => panic!("{:?}", state.peek()),
            };
            // the '+/-' unary operators should bind less tightly than
            // the '**' exponent operator
            let prec = state.prec(TokenKind::Punctuator(Punctuator::Star2)) - PREC_STEP / 2;
            mkunop(state, prec, op)
        }),
        (&["@"], |state| {
            // For now let '@' have same precedence as '-/+/!'
            let prec = state.prec(TokenKind::Punctuator(Punctuator::Star2)) - PREC_STEP / 2;
            let (offset, lineno) = state.pos();
            state.gettok();
            let expr = state.expr(prec)?;
            let kind = expr.kind();
            Ok(Expression::new(
                offset,
                lineno,
                match expr.data_move() {
                    ExpressionData::String(s) => ExpressionData::MutableString(s),
                    ExpressionData::ListDisplay(exprs) => ExpressionData::MutableListDisplay(exprs),
                    ExpressionData::MapDisplay(pairs) => ExpressionData::MutableMapDisplay(pairs),
                    _ => {
                        return Err(ParseError {
                            offset,
                            lineno,
                            kind: ParseErrorKind::ExpectedPotentiallyMutableExpression(kind),
                        })
                    }
                },
            ))
        }),
    ];

    let mut ret = vec![None; TokenKind::LEN];

    for (keys, f) in entries {
        for key in keys {
            let id = TokenKind::from_str(key)
                .expect("Invalid TokenKind in genprefix")
                .id();
            assert!(ret[id].is_none());
            ret[id] = Some(f);
        }
    }

    ret
}

/// returns (infix-table, precedence-table) pair
/// The infix table maps
///     token kinds to a <parsing callback> that can parse
///     the rest of the expression given the left hand side
fn geninfix() -> (
    Vec<Option<fn(&mut ParserState, Expression, Prec) -> Result<Expression, ParseError>>>,
    Vec<Prec>,
) {
    let entries: &[&[(
        &[&str],
        fn(&mut ParserState, Expression, Prec) -> Result<Expression, ParseError>,
    )]] = &[
        &[(&["="], |state, lhs, prec| {
            let (offset, lineno) = state.pos();
            state.gettok();
            let rhs = state.expr(prec - 1)?;
            Ok(Expression::new(
                offset,
                lineno,
                ExpressionData::Assign(lhs.into(), rhs.into()),
            ))
        })],
        &[(&["or"], |state, lhs, prec| {
            mkbinop(state, lhs, prec, Binop::Or)
        })],
        &[(&["and"], |state, lhs, prec| {
            mkbinop(state, lhs, prec, Binop::And)
        })],
        &[
            (&["<"], |state, lhs, prec| {
                mkbinop(state, lhs, prec, Binop::Lt)
            }),
            (&["<="], |state, lhs, prec| {
                mkbinop(state, lhs, prec, Binop::Le)
            }),
            (&[">"], |state, lhs, prec| {
                mkbinop(state, lhs, prec, Binop::Gt)
            }),
            (&[">="], |state, lhs, prec| {
                mkbinop(state, lhs, prec, Binop::Ge)
            }),
            (&["=="], |state, lhs, prec| {
                mkbinop(state, lhs, prec, Binop::Eq)
            }),
            (&["!="], |state, lhs, prec| {
                mkbinop(state, lhs, prec, Binop::Ne)
            }),
            (&["is"], |state, lhs, prec| {
                let (offset, lineno) = state.pos();
                state.gettok();
                let op = if state.consume(TokenKind::Punctuator(Punctuator::Not)) {
                    Binop::IsNot
                } else {
                    Binop::Is
                };
                let rhs = state.expr(prec)?;
                Ok(Expression::new(
                    offset,
                    lineno,
                    ExpressionData::Binop(op, lhs.into(), rhs.into()),
                ))
            }),
        ],
        &[
            (&["+"], |state, lhs, prec| {
                mkbinop(state, lhs, prec, Binop::Add)
            }),
            (&["-"], |state, lhs, prec| {
                mkbinop(state, lhs, prec, Binop::Sub)
            }),
        ],
        &[
            (&["%"], |state, lhs, prec| {
                mkbinop(state, lhs, prec, Binop::Rem)
            }),
            (&["*"], |state, lhs, prec| {
                mkbinop(state, lhs, prec, Binop::Mul)
            }),
            (&["/"], |state, lhs, prec| {
                mkbinop(state, lhs, prec, Binop::Div)
            }),
            (&["//"], |state, lhs, prec| {
                mkbinop(state, lhs, prec, Binop::TruncDiv)
            }),
        ],
        &[(&["**"], |state, lhs, prec| {
            mkbinop(state, lhs, prec - 1, Binop::Pow)
        })],
        &[
            (&["["], |state, lhs, _prec| {
                let (offset, lineno) = state.pos();
                state.gettok();
                let expr = state.expr(0)?;
                state.expect(TokenKind::Punctuator(Punctuator::RBracket))?;
                Ok(Expression::new(
                    offset,
                    lineno,
                    ExpressionData::Subscript(lhs.into(), expr.into()),
                ))
            }),
            (&["("], |state, lhs, _prec| {
                let (offset, lineno) = state.pos();
                let arglist = state.args()?;
                match lhs.data() {
                    ExpressionData::Attribute(..) => match lhs.data_move() {
                        ExpressionData::Attribute(owner, name) => Ok(Expression::new(
                            offset,
                            lineno,
                            ExpressionData::MethodCall(owner, name, arglist),
                        )),
                        _ => panic!("FUBAR"),
                    },
                    _ => Ok(Expression::new(
                        offset,
                        lineno,
                        ExpressionData::FunctionCall(lhs.into(), arglist),
                    )),
                }
            }),
            (&["."], |state, lhs, _prec| {
                let (offset, lineno) = state.pos();
                state.gettok();
                let name = state.expect_name()?;
                Ok(Expression::new(
                    offset,
                    lineno,
                    ExpressionData::Attribute(lhs.into(), name.into()),
                ))
            }),
            (&["::"], |state, lhs, _prec| {
                let (offset, lineno) = state.pos();
                state.gettok();
                let name = state.expect_name()?;
                Ok(Expression::new(
                    offset,
                    lineno,
                    ExpressionData::StaticAttribute(lhs.into(), name.into()),
                ))
            }),
        ],
    ];

    let mut infixtable = vec![None; TokenKind::LEN];
    let mut prectable = vec![-1; TokenKind::LEN];

    for (i, preclevel) in entries.iter().enumerate() {
        let prec = (i + 1) as Prec * PREC_STEP;
        for (opstrs, f) in preclevel.iter() {
            for opstr in opstrs.iter() {
                let op = TokenKind::from_str(opstr).expect("Missing token kind");
                let key = op.id();
                assert_eq!(prectable[key], -1);
                prectable[key] = prec;
                assert!(infixtable[key].is_none());
                infixtable[key] = Some(*f);
            }
        }
    }

    (infixtable, prectable)
}

/// for making one token expressions
fn mk1tokexpr(state: &mut ParserState, data: ExpressionData) -> Result<Expression, ParseError> {
    let (offset, lineno) = state.pos();
    state.gettok();
    Ok(Expression::new(offset, lineno, data))
}

fn mkunop(state: &mut ParserState, prec: Prec, op: Unop) -> Result<Expression, ParseError> {
    let (offset, lineno) = state.pos();
    state.gettok();
    let expr = state.expr(prec)?;
    Ok(Expression::new(
        offset,
        lineno,
        ExpressionData::Unop(op, expr.into()),
    ))
}

fn mkbinop(
    state: &mut ParserState,
    lhs: Expression,
    prec: Prec,
    op: Binop,
) -> Result<Expression, ParseError> {
    let (offset, lineno) = state.pos();
    state.gettok();
    let rhs = state.expr(prec)?;
    Ok(Expression::new(
        offset,
        lineno,
        ExpressionData::Binop(op, lhs.into(), rhs.into()),
    ))
}

fn get_simple_assignment_name(expr: &Expression) -> Option<&RcStr> {
    match expr.data() {
        ExpressionData::Assign(target, _) => match target.data() {
            ExpressionData::Name(name) => Some(name),
            _ => None,
        },
        _ => None,
    }
}

struct InterpretationError {
    offset: usize,
    kind: InterpretationErrorKind,
}

enum InterpretationErrorKind {
    InvalidEscape(String),
    EscapeAtEndOfString,
}

fn interpret_string(s: &str) -> Result<String, InterpretationError> {
    enum Mode {
        Normal,
        Escape,
    }

    let mut ret = String::new();
    let mut offset = 0;
    let mut mode = Mode::Normal;
    for c in s.chars() {
        match mode {
            Mode::Normal => {
                if c == '\\' {
                    mode = Mode::Escape;
                } else {
                    ret.push(c);
                }
            }
            Mode::Escape => match c {
                '\\' => {
                    mode = Mode::Normal;
                    ret.push('\\');
                }
                'n' => {
                    mode = Mode::Normal;
                    ret.push('\n');
                }
                't' => {
                    mode = Mode::Normal;
                    ret.push('\t');
                }
                '"' => {
                    mode = Mode::Normal;
                    ret.push('"');
                }
                '\'' => {
                    mode = Mode::Normal;
                    ret.push('\'');
                }
                _ => {
                    return Err(InterpretationError {
                        offset,
                        kind: InterpretationErrorKind::InvalidEscape(c.to_string()),
                    })
                }
            },
        }
        offset += c.len_utf8();
    }

    match mode {
        Mode::Normal => (),
        Mode::Escape => {
            return Err(InterpretationError {
                offset,
                kind: InterpretationErrorKind::EscapeAtEndOfString,
            });
        }
    }

    Ok(ret)
}

fn assign_name(name: RcStr, expr: Expression) -> Expression {
    let offset = expr.offset();
    let lineno = expr.lineno();
    Expression::new(
        offset,
        lineno,
        ExpressionData::Assign(
            Expression::new(offset, lineno, ExpressionData::Name(name)).into(),
            expr.into(),
        ),
    )
}
