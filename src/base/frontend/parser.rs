use crate::ArgSpec;
use crate::Args;
use crate::AssignTarget;
use crate::AssignTargetDesc;
use crate::Binop;
use crate::ConstVal;
use crate::Error;
use crate::Expr;
use crate::ExprDesc;
use crate::LogicalBinop;
use crate::Mark;
use crate::ModuleDisplay;
use crate::Punctuator;
use crate::RcStr;
use crate::Result;
use crate::Source;
use crate::Token;
use crate::TokenKind;
use crate::Unop;
use std::convert::TryFrom;
use std::rc::Rc;

const PREC_STEP: i32 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ParameterKind {
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

pub(crate) struct Parser {
    prectable: Vec<Prec>,
    prefix_table: Vec<Option<for<'a> fn(&mut ParserState<'a>) -> Result<Expr>>>,
    infix_table: Vec<Option<fn(&mut ParserState, Expr, Prec) -> Result<Expr>>>,
}

impl Parser {
    pub(crate) fn new() -> Parser {
        let (infix_table, prectable) = geninfix();
        Parser {
            prectable,
            prefix_table: genprefix(),
            infix_table,
        }
    }

    pub fn parse_tokens<'a>(
        &self,
        source: Rc<Source>,
        tokens: Vec<Token<'a>>,
        posinfo: Vec<(usize, usize)>,
    ) -> Result<ModuleDisplay> {
        let mut state = ParserState {
            source,
            i: 0,
            tokens,
            posinfo,
            prectable: &self.prectable,
            prefix_table: &self.prefix_table,
            infix_table: &self.infix_table,
        };
        state.parse()
    }
}

type Prec = i32;

struct ParserState<'a> {
    source: Rc<Source>,
    i: usize,
    tokens: Vec<Token<'a>>,
    posinfo: Vec<(usize, usize)>,
    prectable: &'a Vec<Prec>,
    prefix_table: &'a Vec<Option<fn(&mut ParserState) -> Result<Expr>>>,
    infix_table: &'a Vec<Option<fn(&mut ParserState, Expr, Prec) -> Result<Expr>>>,
}

impl<'a> ParserState<'a> {
    fn mark(&self) -> Mark {
        Mark::new(
            self.source.clone(),
            self.posinfo[self.i].0,
            self.posinfo[self.i].1,
        )
    }

    fn peek(&self) -> Token<'a> {
        self.tokens[self.i]
    }

    fn peek1(&self) -> Option<Token<'a>> {
        self.tokens.get(self.i + 1).cloned()
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

    fn expect(&mut self, expected: TokenKind) -> Result<Token<'a>> {
        if self.peek().kind() == expected {
            Ok(self.gettok())
        } else {
            let mark = self.mark();
            Err(Error::rt(
                format!(
                    "Invalid token: expected {:?}, but got {:?}",
                    expected,
                    self.peek().kind()
                )
                .into(),
                vec![mark],
            ))
        }
    }

    fn expect_name(&mut self) -> Result<&'a str> {
        Ok(self.expect(TokenKind::Name)?.name().unwrap())
    }

    fn consume_docstring(&mut self) -> Option<RcStr> {
        if self.at_string() {
            Some(self.expect_string().unwrap())
        } else {
            None
        }
    }

    fn expect_delim(&mut self) -> Result<()> {
        if self.at_delim() {
            self.skip_delim();
            Ok(())
        } else {
            let mark = self.mark();
            Err(Error::rt(
                format!("Expected delimiter, but got {:?}", self.peek().kind()).into(),
                vec![mark],
            ))
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

    fn expect_string(&mut self) -> Result<RcStr> {
        match self.peek() {
            Token::NormalString(s) => {
                self.gettok();
                let raw_string = s;
                match interpret_string(raw_string) {
                    Ok(value) => Ok(value.into()),
                    Err(error) => {
                        let InterpretationError { offset, kind } = error;
                        let mark = self.mark();
                        let mark = Mark::new(mark.source().clone(), mark.pos() + offset, 0);
                        Err(Error::rt(format!("{:?}", kind).into(), vec![mark]))
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
                let mark = self.mark();
                Err(Error::rt(
                    format!("Expected string but got {:?}", self.peek().kind()).into(),
                    vec![mark],
                ))
            }
        }
    }

    fn parse(&mut self) -> Result<ModuleDisplay> {
        let mark = self.mark();

        // check for a docstring
        self.skip_delim();
        let doc = if self.at_string() {
            if let Some(Token::Newline(_)) = self.peek1() {
                Some(self.expect_string()?)
            } else {
                None
            }
        } else {
            None
        };

        let mut exprs = Vec::new();
        self.skip_delim();
        while self.peek().kind() != TokenKind::EOF {
            exprs.push(self.stmt()?);
            self.expect_delim()?;
        }
        let body = Expr::new(mark, ExprDesc::Block(exprs));
        Ok(ModuleDisplay::new(self.source.name().clone(), doc, body))
    }

    fn prec(&self, kind: TokenKind) -> Prec {
        self.prectable[kind.id()]
    }

    fn cprec(&self) -> Prec {
        self.prec(self.peek().kind())
    }

    fn block_ex(&mut self, nil_appended: bool) -> Result<(Option<RcStr>, Expr)> {
        let mark = self.mark();
        self.expect(TokenKind::Punctuator(Punctuator::LBrace))?;
        let mut exprs = Vec::new();
        self.skip_delim();
        while !self.consume(TokenKind::Punctuator(Punctuator::RBrace)) {
            exprs.push(self.stmt()?);
            self.expect_delim()?;
        }
        let docstr = if let Some(ExprDesc::String(s)) = exprs.get(0).map(|e| e.desc()) {
            Some(s.clone())
        } else {
            None
        };
        if nil_appended {
            exprs.push(Expr::new(mark.clone(), ExprDesc::Nil));
        }
        Ok((docstr, Expr::new(mark, ExprDesc::Block(exprs))))
    }

    fn block(&mut self) -> Result<Expr> {
        Ok(self.block_ex(false)?.1)
    }

    fn block_with_doc(&mut self) -> Result<(Option<RcStr>, Expr)> {
        self.block_ex(false)
    }

    fn nil_appended_block_with_doc(&mut self) -> Result<(Option<RcStr>, Expr)> {
        self.block_ex(true)
    }

    /// parse an expression with the given precedence
    fn expr(&mut self, prec: Prec) -> Result<Expr> {
        let mut expr = self.prefix()?;
        while self.cprec() > prec {
            expr = self.infix(expr)?;
        }
        Ok(expr)
    }

    fn import_module_name(&mut self) -> Result<(String, &str)> {
        let mut name = String::new();
        while self.consume(TokenKind::Punctuator(Punctuator::Dot)) {
            name.push('.');
        }
        let mut last_part = self.expect_name()?;
        name.push_str(last_part);
        while self.consume(TokenKind::Punctuator(Punctuator::Dot)) {
            name.push('.');
            last_part = self.expect_name()?;
            name.push_str(last_part);
        }
        Ok((name, last_part))
    }

    /// parse a statement
    /// a statement is basically an expression, except that named functions
    /// will automatically be assigned to a variable of the same name
    fn stmt(&mut self) -> Result<Expr> {
        let expr = self.expr(0)?;

        // If we see an assignment followed by a '#' string on the next line,
        // we assume that the string is meant to be a doc for the assignment
        if let Some(name) = Self::get_assign_name(&expr) {
            if let Some(doc) = self.followup_doc()? {
                let mark = self.mark();
                return Ok(Expr::new(mark, ExprDesc::AssignDoc(expr.into(), name, doc)));
            }
        }

        // functions and classes declared at the statement level will always be
        // included in a module's documentation regardless of whether they have
        // explicit docs.
        // This differs form with values explicitly assigned with '=', that are
        // only included if there are explicit docs.
        // If you want to avoid including a function or class in a module's docs,
        // you can do something like
        //
        //   some_func = def some_func() = { .. }
        //
        // without any followup docs.
        //
        fn assign_with_doc(expr: Expr, name: RcStr, docstr: Option<RcStr>) -> Expr {
            let mark = expr.mark().clone();
            let assign_expr = assign_name(name.clone(), expr);
            let docstr = docstr.unwrap_or("".into());
            Expr::new(mark, ExprDesc::AssignDoc(assign_expr.into(), name, docstr))
        }

        Ok(match expr.desc() {
            ExprDesc::Function {
                name: Some(name),
                docstr,
                ..
            } => {
                let name = name.clone();
                let docstr = docstr.clone();
                assign_with_doc(expr, name, docstr)
            }
            ExprDesc::Class { name, docstr, .. } => {
                let name = name.clone();
                let docstr = docstr.clone();
                assign_with_doc(expr, name, docstr)
            }
            _ => expr,
        })
    }

    fn get_assign_name(expr: &Expr) -> Option<RcStr> {
        if let ExprDesc::Assign(target, _) = expr.desc() {
            if let AssignTargetDesc::Name(name) = target.desc() {
                return Some(name.clone());
            }
        }
        None
    }

    fn followup_doc(&mut self) -> Result<Option<RcStr>> {
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

    fn prefix(&mut self) -> Result<Expr> {
        let key = self.peek().kind().id();
        if let Some(f) = self.prefix_table[key] {
            f(self)
        } else {
            let mark = self.mark();
            Err(Error::rt(
                format!("Expected expression but got {:?}", self.peek().kind()).into(),
                vec![mark],
            ))
        }
    }

    fn infix(&mut self, expr: Expr) -> Result<Expr> {
        let key = self.peek().kind().id();
        let prec = self.cprec();
        self.infix_table[key].unwrap()(self, expr, prec)
    }

    fn params(&mut self) -> Result<ArgSpec> {
        self.expect(TokenKind::Punctuator(Punctuator::LParen))?;
        let mut req = Vec::new(); // required params
        let mut opt = Vec::new(); // optional params
        let mut variadic = None;
        let mut keywords: Option<RcStr> = None;
        let mut last_kind = ParameterKind::Required;
        while !self.consume(TokenKind::Punctuator(Punctuator::RParen)) {
            let mark = self.mark();
            let kind = match self.gettok() {
                Token::Name(name) => {
                    if self.consume(TokenKind::Punctuator(Punctuator::Eq)) {
                        // optional parameter
                        let expr = self.expr(0)?;
                        opt.push((name.into(), to_constval(expr)?));
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
                    return Err(Error::rt(
                        format!("Expected parameter but got {:?}", token).into(),
                        vec![mark],
                    ));
                }
            };
            if last_kind > kind {
                return Err(Error::rt(
                    format!("InvalidParameterOrder").into(),
                    vec![mark],
                ));
            }
            if !kind.multiple_allowed() && last_kind == kind {
                return Err(Error::rt(
                    format!("IllegalDuplicateParameterKind({:?})", kind).into(),
                    vec![mark],
                ));
            }
            last_kind = kind;
            if !self.consume(TokenKind::Punctuator(Punctuator::Comma)) {
                self.expect(TokenKind::Punctuator(Punctuator::RParen))?;
                break;
            }
        }
        Ok(ArgSpec::new(req, opt, variadic, keywords))
    }

    fn args(&mut self) -> Result<Args> {
        let mark = self.mark();
        self.expect(TokenKind::Punctuator(Punctuator::LParen))?;
        let mut pos = Vec::new();
        let mut key = Vec::new();
        let mut variadic = None;
        let mut kwtable = None;
        while !self.consume(TokenKind::Punctuator(Punctuator::RParen)) {
            let mark = self.mark();
            if self.consume(TokenKind::Punctuator(Punctuator::Star)) {
                // kwtables have to come after vararg arguments
                if kwtable.is_some() {
                    return Err(Error::rt(
                        format!("Illegal argument order").into(),
                        vec![mark],
                    ));
                }
                if variadic.is_some() {
                    return Err(Error::rt(
                        format!("Multiple variadic arguments are not allowed").into(),
                        vec![mark],
                    ));
                }
                variadic = Some(self.expr(0)?);
            } else if self.consume(TokenKind::Punctuator(Punctuator::Star2)) {
                if kwtable.is_some() {
                    return Err(Error::rt(
                        format!("Multiple keyword talbe arguments are not allowed").into(),
                        vec![mark],
                    ));
                }
                kwtable = Some(self.expr(0)?);
            } else if self.peek().kind() == TokenKind::Name
                && self.tokens.get(self.i + 1).map(|t| t.kind())
                    == Some(TokenKind::Punctuator(Punctuator::Eq))
            {
                // variadic and kwtables have to come after vararg arguments
                if variadic.is_some() || kwtable.is_some() {
                    return Err(Error::rt(
                        format!("Illegal argument order").into(),
                        vec![mark],
                    ));
                }
                let name = self.expect_name()?;
                self.expect(TokenKind::Punctuator(Punctuator::Eq))?;
                let expr = self.expr(0)?;
                key.push((name.into(), expr));
            } else {
                // keyword, variadic and kwtables have to come after vararg arguments
                if !key.is_empty() || variadic.is_some() || kwtable.is_some() {
                    return Err(Error::rt(
                        format!("Illegal argument order").into(),
                        vec![mark],
                    ));
                }
                pos.push(self.expr(0)?);
            }

            if !self.consume(TokenKind::Punctuator(Punctuator::Comma)) {
                self.expect(TokenKind::Punctuator(Punctuator::RParen))?;
                break;
            }
        }
        if variadic.is_some() || kwtable.is_some() {
            return Err(Error::rt(
                format!("*varargs/**kw style arguments are not supported").into(),
                vec![mark],
            ));
        }
        Ok(Args::new(pos, key))
    }
}

/// generate the prefix table
/// a prefix table maps
///     token kinds to a <parsing callback> that can parse an expression
///     that starts with the given token kind
fn genprefix() -> Vec<Option<fn(&mut ParserState) -> Result<Expr>>> {
    let entries: Vec<(&[&'static str], fn(&mut ParserState) -> Result<Expr>)> = vec![
        (&["nil"], |state: &mut ParserState| {
            mk1tokexpr(state, ExprDesc::Nil)
        }),
        (&["true"], |state: &mut ParserState| {
            mk1tokexpr(state, ExprDesc::Bool(true))
        }),
        (&["false"], |state: &mut ParserState| {
            mk1tokexpr(state, ExprDesc::Bool(false))
        }),
        (&["Int"], |state: &mut ParserState| {
            let value = state.peek().int().unwrap();
            mk1tokexpr(state, ExprDesc::Number(value as f64))
        }),
        (&["Float"], |state: &mut ParserState| {
            let value = state.peek().float().unwrap();
            mk1tokexpr(state, ExprDesc::Number(value))
        }),
        (
            &["NormalString", "RawString", "LineString"],
            |state: &mut ParserState| {
                let mark = state.mark();
                let s = state.expect_string()?;
                Ok(Expr::new(mark, ExprDesc::String(s.into())))
            },
        ),
        (&["Name"], |state: &mut ParserState| {
            let name = state.peek().name().unwrap();
            mk1tokexpr(state, ExprDesc::Name(name.into()))
        }),
        (&["{"], |state: &mut ParserState| state.block()),
        (&["("], |state: &mut ParserState| {
            let mark = state.mark();
            state.gettok();
            let expr = state.expr(0)?;
            state.expect(TokenKind::Punctuator(Punctuator::RParen))?;
            Ok(Expr::new(mark, ExprDesc::Parentheses(expr.into())))
        }),
        (&["["], |state: &mut ParserState| {
            let mark = state.mark();
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
                Ok(Expr::new(mark, ExprDesc::Map(vec![])))
            } else if state.consume(TokenKind::Punctuator(Punctuator::RBracket)) {
                // Empty list
                Ok(Expr::new(mark, ExprDesc::List(vec![])))
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
                    Ok(Expr::new(mark, ExprDesc::Map(pairs)))
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
                    Ok(Expr::new(mark, ExprDesc::List(exprs)))
                }
            }
        }),
        (&["if"], |state: &mut ParserState| {
            let mark = state.mark();
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
            Ok(Expr::new(mark, ExprDesc::If(pairs, other)))
        }),
        (&["switch"], |state: &mut ParserState| {
            let mark = state.mark();
            state.gettok();
            let target = state.expr(0)?;
            state.expect(TokenKind::Punctuator(Punctuator::LBrace))?;
            let mut pairs = Vec::new();
            let mut other = None;
            state.skip_delim();
            while !state.consume(TokenKind::Punctuator(Punctuator::RBrace)) {
                if state.consume(TokenKind::Punctuator(Punctuator::Arrow)) {
                    other = Some(state.expr(0)?.into());
                    state.expect_delim()?;
                    state.expect(TokenKind::Punctuator(Punctuator::RBrace))?;
                    break;
                } else {
                    let match_ = state.expr(0)?;
                    state.expect(TokenKind::Punctuator(Punctuator::Arrow))?;
                    let body = state.expr(0)?;
                    pairs.push((match_, body));
                    state.expect_delim()?;
                }
            }
            Ok(Expr::new(
                mark,
                ExprDesc::Switch(target.into(), pairs, other),
            ))
        }),
        (&["for"], |state: &mut ParserState| {
            let mark = state.mark();
            state.gettok();
            let target = to_target(state.expr(0)?)?;
            state.expect(TokenKind::Punctuator(Punctuator::In))?;
            let iterable = state.expr(0)?.into();
            let body = state.block()?.into();
            Ok(Expr::new(mark, ExprDesc::For(target, iterable, body)))
        }),
        (&["while"], |state: &mut ParserState| {
            let mark = state.mark();
            state.gettok();
            let cond = state.expr(0)?.into();
            let body = state.block()?.into();
            Ok(Expr::new(mark, ExprDesc::While(cond, body)))
        }),
        (&["del"], |state: &mut ParserState| {
            let mark = state.mark();
            state.gettok();
            let varname = state.expect_name()?;
            Ok(Expr::new(mark, ExprDesc::Del(varname.into())))
        }),
        (&["nonlocal"], |state: &mut ParserState| {
            let mark = state.mark();
            state.gettok();
            let mut names = Vec::new();
            while state.peek().kind() == TokenKind::Name {
                names.push(state.expect_name()?.into());
                if !state.consume(TokenKind::Punctuator(Punctuator::Comma)) {
                    break;
                }
            }
            Ok(Expr::new(mark, ExprDesc::Nonlocal(names)))
        }),
        (&["yield"], |state: &mut ParserState| {
            let mark = state.mark();
            state.gettok();
            let expr = state.expr(0)?;
            Ok(Expr::new(mark, ExprDesc::Yield(expr.into())))
        }),
        (&["return"], |state: &mut ParserState| {
            let mark = state.mark();
            state.gettok();
            let expr = if state.at_delim() {
                None
            } else {
                Some(state.expr(0)?.into())
            };
            Ok(Expr::new(mark, ExprDesc::Return(expr)))
        }),
        (&["def"], |state: &mut ParserState| {
            let mark = state.mark();
            state.gettok();

            // 'def break' is special syntax to indicate a breakpoint
            if state.consume(TokenKind::Punctuator(Punctuator::Break)) {
                return Ok(Expr::new(mark, ExprDesc::BreakPoint));
            }

            // otherwise we're dealing with a function definition
            let is_generator = state.consume(TokenKind::Punctuator(Punctuator::Star));
            let name = if state.peek().kind() == TokenKind::Name {
                Some(state.expect_name()?.into())
            } else {
                None
            };
            let params = if state.peek().kind() == TokenKind::Punctuator(Punctuator::LParen) {
                state.params()?
            } else {
                ArgSpec::empty()
            };
            let (docstr, body) = if state.consume(TokenKind::Punctuator(Punctuator::Eq)) {
                if state.peek() == Token::Punctuator(Punctuator::LBrace) {
                    state.block_with_doc()?
                } else {
                    (None, state.expr(0)?)
                }
            } else {
                state.nil_appended_block_with_doc()?
            };
            Ok(Expr::new(
                mark,
                ExprDesc::Function {
                    is_generator,
                    name,
                    params,
                    docstr,
                    body: body.into(),
                    varspec: None,
                },
            ))
        }),
        (&["class"], |state: &mut ParserState| {
            let mark = state.mark();
            state.expect(TokenKind::Punctuator(Punctuator::Class))?;
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
            let (docstr, methods, static_methods) = {
                let mut docstring = None;
                let mut methods = Vec::new();
                let mut static_methods = Vec::new();
                if state.consume(TokenKind::Punctuator(Punctuator::LBrace)) {
                    state.skip_delim();

                    docstring = state.consume_docstring();

                    state.skip_delim();

                    while !state.consume(TokenKind::Punctuator(Punctuator::RBrace)) {
                        if state.consume(TokenKind::Punctuator(Punctuator::New)) {
                            let mark = state.mark();
                            let params = state.params()?;
                            state.expect(TokenKind::Punctuator(Punctuator::Eq))?;
                            let (docstr, body) =
                                if state.peek() == Token::Punctuator(Punctuator::LBrace) {
                                    state.block_with_doc()?
                                } else {
                                    (None, state.expr(0)?)
                                };
                            let member = Expr::new(
                                mark,
                                ExprDesc::Function {
                                    is_generator: false,
                                    name: Some("__call".into()),
                                    params,
                                    docstr,
                                    body: body.into(),
                                    varspec: None,
                                },
                            );
                            static_methods.push((RcStr::from("__call"), member));
                        } else {
                            let mark = state.mark();
                            let out = if state.consume(TokenKind::Punctuator(Punctuator::Static)) {
                                &mut static_methods
                            } else {
                                &mut methods
                            };
                            let stmt = state.stmt()?;
                            let (name, member) = match break_assignment(stmt) {
                                Some((name, member)) => (name, member),
                                None => {
                                    return Err(Error::rt(
                                        format!("Expected class member").into(),
                                        vec![mark],
                                    ))
                                }
                            };
                            out.push((name, member));
                        }
                        state.expect_delim()?;
                    }
                }
                (docstring, methods, static_methods)
            };
            Ok(Expr::new(
                mark,
                ExprDesc::Class {
                    name: short_name,
                    bases,
                    docstr,
                    methods,
                    static_methods,
                },
            ))
        }),
        (&["import", "from"], |state: &mut ParserState| {
            let mark = state.mark();
            let from_ = if state.consume(TokenKind::Punctuator(Punctuator::From)) {
                true
            } else {
                state.expect(TokenKind::Punctuator(Punctuator::Import))?;
                false
            };
            let (name, last_part) = state.import_module_name()?;

            let mut last_part = RcStr::from(last_part);

            let field = if from_ {
                state.expect(TokenKind::Punctuator(Punctuator::Import))?;
                last_part = RcStr::from(state.expect_name()?);
                Some(last_part.clone())
            } else {
                None
            };

            let alias = if state.consume(TokenKind::Punctuator(Punctuator::As)) {
                RcStr::from(state.expect_name()?)
            } else {
                last_part
            };
            let raw_import = Expr::new(mark.clone(), ExprDesc::Import(name.into()));
            let field_applied = match field {
                Some(field) => Expr::new(mark, ExprDesc::Attr(raw_import.into(), field.into())),
                None => raw_import,
            };
            Ok(assign_name(alias, field_applied))
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
    Vec<Option<fn(&mut ParserState, Expr, Prec) -> Result<Expr>>>,
    Vec<Prec>,
) {
    let entries: &[&[(&[&str], fn(&mut ParserState, Expr, Prec) -> Result<Expr>)]] = &[
        &[
            (&["="], |state, lhs, prec| {
                let mark = state.mark();
                state.gettok();
                let rhs = state.expr(prec - 1)?;
                Ok(Expr::new(
                    mark,
                    ExprDesc::Assign(to_target(lhs)?, rhs.into()),
                ))
            }),
            (
                &["+=", "-=", "*=", "/=", "//=", "%=", "**="],
                |state, lhs, prec| {
                    let mark = state.mark();
                    let op = match state.gettok() {
                        Token::Punctuator(Punctuator::PlusEq) => Binop::Add,
                        Token::Punctuator(Punctuator::MinusEq) => Binop::Sub,
                        Token::Punctuator(Punctuator::StarEq) => Binop::Mul,
                        Token::Punctuator(Punctuator::SlashEq) => Binop::Div,
                        Token::Punctuator(Punctuator::DoubleSlashEq) => Binop::TruncDiv,
                        Token::Punctuator(Punctuator::RemEq) => Binop::Rem,
                        Token::Punctuator(Punctuator::Star2Eq) => Binop::Pow,
                        tok => panic!("Unhandled augassign binop {:?}", tok),
                    };
                    let rhs = state.expr(prec - 1)?;
                    Ok(Expr::new(
                        mark,
                        ExprDesc::AugAssign(to_target(lhs)?, op, rhs.into()),
                    ))
                },
            ),
        ],
        &[(&["or"], |state, lhs, prec| {
            mklbinop(state, lhs, prec, LogicalBinop::Or)
        })],
        &[(&["and"], |state, lhs, prec| {
            mklbinop(state, lhs, prec, LogicalBinop::And)
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
                let mark = state.mark();
                state.gettok();
                let op = if state.consume(TokenKind::Punctuator(Punctuator::Not)) {
                    Binop::IsNot
                } else {
                    Binop::Is
                };
                let rhs = state.expr(prec)?;
                Ok(Expr::new(mark, ExprDesc::Binop(op, lhs.into(), rhs.into())))
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
                let mark = state.mark();
                state.gettok();
                if state.consume(TokenKind::Punctuator(Punctuator::Colon)) {
                    // slice with start omitted
                    let end = if state.peek() == Token::Punctuator(Punctuator::RBracket) {
                        None
                    } else {
                        Some(state.expr(0)?.into())
                    };
                    state.expect(TokenKind::Punctuator(Punctuator::RBracket))?;
                    Ok(Expr::new(mark, ExprDesc::Slice(lhs.into(), None, end)))
                } else {
                    let expr = state.expr(0)?;
                    if state.consume(TokenKind::Punctuator(Punctuator::Colon)) {
                        // slice with start present
                        let start = Some(expr.into());
                        let end = if state.peek() == Token::Punctuator(Punctuator::RBracket) {
                            None
                        } else {
                            Some(state.expr(0)?.into())
                        };
                        state.expect(TokenKind::Punctuator(Punctuator::RBracket))?;
                        Ok(Expr::new(mark, ExprDesc::Slice(lhs.into(), start, end)))
                    } else {
                        // a simple subscript expression
                        state.expect(TokenKind::Punctuator(Punctuator::RBracket))?;
                        Ok(Expr::new(
                            mark,
                            ExprDesc::Subscript(lhs.into(), expr.into()),
                        ))
                    }
                }
            }),
            (&["("], |state, lhs, _prec| {
                let mark = state.mark();
                let arglist = state.args()?;
                match lhs.desc() {
                    ExprDesc::Attr(..) => match lhs.unpack() {
                        (mark, ExprDesc::Attr(owner, name)) => {
                            Ok(Expr::new(mark, ExprDesc::CallMethod(owner, name, arglist)))
                        }
                        _ => panic!("FUBAR"),
                    },
                    _ => Ok(Expr::new(mark, ExprDesc::CallFunction(lhs.into(), arglist))),
                }
            }),
            (&["."], |state, lhs, _prec| {
                let mark = state.mark();
                state.gettok();
                let name = state.expect_name()?;
                Ok(Expr::new(mark, ExprDesc::Attr(lhs.into(), name.into())))
            }),
            (&["::"], |state, lhs, _prec| {
                // TODO: Consider making behavior for this different
                let mark = state.mark();
                state.gettok();
                let name = state.expect_name()?;
                Ok(Expr::new(mark, ExprDesc::Attr(lhs.into(), name.into())))
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
fn mk1tokexpr(state: &mut ParserState, data: ExprDesc) -> Result<Expr> {
    let mark = state.mark();
    state.gettok();
    Ok(Expr::new(mark, data))
}

fn mkunop(state: &mut ParserState, prec: Prec, op: Unop) -> Result<Expr> {
    let mark = state.mark();
    state.gettok();
    let expr = state.expr(prec)?;
    Ok(Expr::new(mark, ExprDesc::Unop(op, expr.into())))
}

fn mkbinop(state: &mut ParserState, lhs: Expr, prec: Prec, op: Binop) -> Result<Expr> {
    let mark = state.mark();
    state.gettok();
    let rhs = state.expr(prec)?;
    Ok(Expr::new(mark, ExprDesc::Binop(op, lhs.into(), rhs.into())))
}

fn mklbinop(state: &mut ParserState, lhs: Expr, prec: Prec, op: LogicalBinop) -> Result<Expr> {
    let mark = state.mark();
    state.gettok();
    let rhs = state.expr(prec)?;
    Ok(Expr::new(
        mark,
        ExprDesc::LogicalBinop(op, lhs.into(), rhs.into()),
    ))
}

fn break_assignment(expr: Expr) -> Option<(RcStr, Expr)> {
    match expr.unpack() {
        (_mark, ExprDesc::Assign(target, expr)) => match target.unpack() {
            (_mark, AssignTargetDesc::Name(name)) => Some((name, *expr)),
            _ => None,
        },
        (_mark, ExprDesc::AssignDoc(expr, _, _)) => break_assignment(*expr),
        _ => None,
    }
}

struct InterpretationError {
    offset: usize,
    kind: InterpretationErrorKind,
}

#[derive(Debug)]
enum InterpretationErrorKind {
    InvalidEscape(String),
    EscapeAtEndOfString,
}

fn interpret_string(s: &str) -> std::result::Result<String, InterpretationError> {
    enum Mode {
        Normal,
        Escape,
        ByteStart,
        ByteFirst(char),
        CharEscapeStart(u32),
        CharEscape(u32, usize),
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
            Mode::ByteStart => mode = Mode::ByteFirst(c),
            Mode::ByteFirst(first) => {
                let snippet = format!("{}{}", first, c);
                let code = match u8::from_str_radix(&snippet, 16) {
                    Ok(code) => code,
                    Err(error) => {
                        return Err(InterpretationError {
                            offset,
                            kind: InterpretationErrorKind::InvalidEscape(format!("{:?}", error)),
                        })
                    }
                };
                ret.push(code as char);
                mode = Mode::Normal;
            }
            Mode::CharEscapeStart(radix) => {
                if c != '{' {
                    return Err(InterpretationError {
                        offset,
                        kind: InterpretationErrorKind::InvalidEscape(
                            format!("Expected '{{' here",),
                        ),
                    });
                }
                mode = Mode::CharEscape(radix, offset + c.len_utf8());
            }
            Mode::CharEscape(radix, start) => {
                if c == '}' {
                    let snippet = &s[start..offset];
                    let code = match u32::from_str_radix(snippet, radix) {
                        Ok(code) => code,
                        Err(error) => {
                            return Err(InterpretationError {
                                offset,
                                kind: InterpretationErrorKind::InvalidEscape(format!(
                                    "{:?}",
                                    error
                                )),
                            })
                        }
                    };
                    match char::try_from(code) {
                        Ok(ch) => ret.push(ch),
                        Err(error) => {
                            return Err(InterpretationError {
                                offset,
                                kind: InterpretationErrorKind::InvalidEscape(format!(
                                    "{:?}",
                                    error
                                )),
                            })
                        }
                    }
                    mode = Mode::Normal;
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
                '\0' => {
                    mode = Mode::Normal;
                    ret.push('\0');
                }
                'x' => {
                    mode = Mode::ByteStart;
                }
                'u' => {
                    // hexadecimal escape
                    mode = Mode::CharEscapeStart(16);
                }
                'o' => {
                    // octal escape
                    mode = Mode::CharEscapeStart(8);
                }
                'd' => {
                    // decimal escape
                    mode = Mode::CharEscapeStart(10);
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
        Mode::Escape
        | Mode::ByteStart
        | Mode::ByteFirst(_)
        | Mode::CharEscapeStart(..)
        | Mode::CharEscape(..) => {
            return Err(InterpretationError {
                offset,
                kind: InterpretationErrorKind::EscapeAtEndOfString,
            });
        }
    }

    Ok(ret)
}

fn assign_name(name: RcStr, expr: Expr) -> Expr {
    let mark = expr.mark().clone();
    Expr::new(
        mark.clone(),
        ExprDesc::Assign(
            AssignTarget::new(mark, AssignTargetDesc::Name(name)),
            expr.into(),
        ),
    )
}

fn to_constval(expr: Expr) -> Result<ConstVal> {
    match expr.desc() {
        ExprDesc::Nil => Ok(ConstVal::Nil),
        ExprDesc::Bool(b) => Ok(ConstVal::Bool(*b)),
        ExprDesc::Number(x) => Ok(ConstVal::Number(*x)),
        ExprDesc::String(x) => Ok(ConstVal::String(x.clone())),
        _ => Err(Error::rt(
            "Expected constant expression".into(),
            vec![expr.mark().clone()],
        )),
    }
}

fn to_target(expr: Expr) -> Result<AssignTarget> {
    let (mark, desc) = expr.unpack();
    let desc = match desc {
        ExprDesc::Name(name) => AssignTargetDesc::Name(name),
        ExprDesc::List(vec) => {
            AssignTargetDesc::List(vec.into_iter().map(to_target).collect::<Result<_>>()?)
        }
        ExprDesc::Attr(owner, name) => AssignTargetDesc::Attr(owner, name),
        _ => {
            return Err(Error::rt(
                "The target expression is not assignable".into(),
                vec![mark],
            ))
        }
    };
    Ok(AssignTarget::new(mark, desc))
}
