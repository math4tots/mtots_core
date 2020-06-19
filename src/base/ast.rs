use crate::ClassKind;
use crate::RcStr;
use crate::Symbol;

pub struct Expression {
    offset: usize, // offset from start of file where this expression was parsed
    lineno: usize,
    data: ExpressionData,
}

impl Expression {
    pub fn new(offset: usize, lineno: usize, data: ExpressionData) -> Expression {
        Expression {
            offset,
            lineno,
            data,
        }
    }
    pub fn lineno(&self) -> usize {
        self.lineno
    }
    pub fn offset(&self) -> usize {
        self.offset
    }
    pub fn data(&self) -> &ExpressionData {
        &self.data
    }
    pub fn data_move(self) -> ExpressionData {
        self.data
    }
    pub fn children_mut(&mut self) -> Vec<&mut Expression> {
        match &mut self.data {
            ExpressionData::Nil => vec![],
            ExpressionData::Bool(_) => vec![],
            ExpressionData::Int(_) => vec![],
            ExpressionData::Float(_) => vec![],
            ExpressionData::Symbol(_) => vec![],
            ExpressionData::String(_) => vec![],
            ExpressionData::MutableString(_) => vec![],
            ExpressionData::Name(_) => vec![],
            ExpressionData::Del(_) => vec![],
            ExpressionData::Nonlocal(_) => vec![],
            ExpressionData::Parentheses(expr) => vec![&mut *expr],
            ExpressionData::Block(ref mut exprs) => exprs2mutrefs(exprs),
            ExpressionData::ListDisplay(ref mut exprs) => exprs2mutrefs(exprs),
            ExpressionData::MapDisplay(ref mut pairs) => pairs2mutrefs(pairs),
            ExpressionData::MutableListDisplay(ref mut exprs) => exprs2mutrefs(exprs),
            ExpressionData::MutableMapDisplay(ref mut pairs) => pairs2mutrefs(pairs),
            ExpressionData::Assign(target, expr) => vec![&mut *target, &mut *expr],
            ExpressionData::AugAssign(target, _, expr) => vec![&mut *target, &mut *expr],
            ExpressionData::AssignWithDoc(assign, _, _) => vec![&mut *assign],
            ExpressionData::If(pairs, other) => join_mut_refs(vec![
                pairs
                    .iter_mut()
                    .map(|(c, b)| vec![c, b])
                    .flatten()
                    .collect(),
                other.iter_mut().map(|other| &mut **other).collect(),
            ]),
            ExpressionData::For(target, iterable, body) => {
                vec![&mut *target, &mut *iterable, &mut *body]
            }
            ExpressionData::While(cond, body) => vec![&mut *cond, &mut *body],
            ExpressionData::Unop(_, expr) => vec![&mut *expr],
            ExpressionData::Binop(_, lhs, rhs) => vec![&mut *lhs, &mut *rhs],
            ExpressionData::Attribute(owner, _) => vec![&mut *owner],
            ExpressionData::StaticAttribute(owner, _) => vec![&mut *owner],
            ExpressionData::Subscript(owner, index) => vec![&mut *owner, &mut *index],
            ExpressionData::Slice(owner, start, end) => join_mut_refs(vec![
                vec![&mut *owner],
                start.iter_mut().map(|e| &mut *e.as_mut()).collect(),
                end.iter_mut().map(|e| &mut *e.as_mut()).collect(),
            ]),
            ExpressionData::FunctionCall(f, arglist) => {
                join_mut_refs(vec![vec![&mut *f], arglist.children_mut()])
            }
            ExpressionData::MethodCall(owner, _, arglist) => {
                join_mut_refs(vec![vec![&mut *owner], arglist.children_mut()])
            }
            ExpressionData::New(arglist) => arglist.children_mut(),
            ExpressionData::FunctionDisplay(_, _, _, defparams, _, _, _, body) => {
                join_mut_refs(vec![
                    defparams.iter_mut().map(|(_, e)| e).collect(),
                    vec![&mut *body],
                ])
            }
            ExpressionData::ClassDisplay(_, _, bases, _, _, methods, static_methods) => {
                join_mut_refs(vec![
                    exprs2mutrefs(bases),
                    methods.iter_mut().map(|(_, e)| e).collect(),
                    static_methods.iter_mut().map(|(_, e)| e).collect(),
                ])
            }
            ExpressionData::ExceptionKindDisplay(_, base, _, _, msgexpr) => join_mut_refs(vec![
                match base {
                    Some(base) => vec![&mut *base],
                    None => vec![],
                },
                vec![&mut *msgexpr],
            ]),
            ExpressionData::Import(..) => vec![],
            ExpressionData::Yield(expr) => vec![&mut *expr],
            ExpressionData::Return(expr) => match expr {
                Some(expr) => vec![&mut *expr],
                None => vec![],
            },
            ExpressionData::BreakPoint => vec![],
        }
    }
    pub fn kind(&self) -> ExpressionKind {
        match &self.data {
            ExpressionData::Nil => ExpressionKind::Nil,
            ExpressionData::Bool(_) => ExpressionKind::Bool,
            ExpressionData::Int(_) => ExpressionKind::Int,
            ExpressionData::Float(_) => ExpressionKind::Float,
            ExpressionData::Symbol(_) => ExpressionKind::Symbol,
            ExpressionData::String(_) => ExpressionKind::String,
            ExpressionData::MutableString(_) => ExpressionKind::MutableString,
            ExpressionData::Name(_) => ExpressionKind::Name,
            ExpressionData::Del(_) => ExpressionKind::Del,
            ExpressionData::Nonlocal(_) => ExpressionKind::Nonlocal,
            ExpressionData::Parentheses(..) => ExpressionKind::Parentheses,
            ExpressionData::Block(..) => ExpressionKind::Block,
            ExpressionData::ListDisplay(..) => ExpressionKind::ListDisplay,
            ExpressionData::MapDisplay(..) => ExpressionKind::MapDisplay,
            ExpressionData::MutableListDisplay(..) => ExpressionKind::MutableListDisplay,
            ExpressionData::MutableMapDisplay(..) => ExpressionKind::MutableMapDisplay,
            ExpressionData::Assign(..) => ExpressionKind::Assign,
            ExpressionData::AugAssign(..) => ExpressionKind::AugAssign,
            ExpressionData::AssignWithDoc(..) => ExpressionKind::AssignWithDoc,
            ExpressionData::If(..) => ExpressionKind::If,
            ExpressionData::For(..) => ExpressionKind::For,
            ExpressionData::While(..) => ExpressionKind::While,
            ExpressionData::Unop(..) => ExpressionKind::Unop,
            ExpressionData::Binop(..) => ExpressionKind::Binop,
            ExpressionData::Attribute(..) => ExpressionKind::Attribute,
            ExpressionData::StaticAttribute(..) => ExpressionKind::StaticAttribute,
            ExpressionData::Subscript(..) => ExpressionKind::Subscript,
            ExpressionData::Slice(..) => ExpressionKind::Slice,
            ExpressionData::FunctionCall(..) => ExpressionKind::FunctionCall,
            ExpressionData::MethodCall(..) => ExpressionKind::MethodCall,
            ExpressionData::New(..) => ExpressionKind::New,
            ExpressionData::FunctionDisplay(..) => ExpressionKind::FunctionDisplay,
            ExpressionData::ClassDisplay(..) => ExpressionKind::ClassDisplay,
            ExpressionData::ExceptionKindDisplay(..) => ExpressionKind::ExceptionKindDisplay,
            ExpressionData::Import(..) => ExpressionKind::Import,
            ExpressionData::Yield(..) => ExpressionKind::Yield,
            ExpressionData::Return(..) => ExpressionKind::Return,
            ExpressionData::BreakPoint => ExpressionKind::BreakPoint,
        }
    }
}

fn exprs2mutrefs(exprs: &mut Vec<Expression>) -> Vec<&mut Expression> {
    let mut ret = vec![];
    for expr in exprs.iter_mut() {
        ret.push(expr);
    }
    ret
}

fn pairs2mutrefs(exprs: &mut Vec<(Expression, Expression)>) -> Vec<&mut Expression> {
    let mut ret = vec![];
    for (key, val) in exprs.iter_mut() {
        ret.push(key);
        ret.push(val);
    }
    ret
}

fn join_mut_refs(vec: Vec<Vec<&mut Expression>>) -> Vec<&mut Expression> {
    let mut ret = vec![];
    for subvec in vec {
        for expr in subvec {
            ret.push(expr);
        }
    }
    ret
}

#[derive(Debug, Clone, Copy)]
pub enum Unop {
    Pos,
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy)]
pub enum Binop {
    Add,
    Sub,
    Mul,
    Div,
    TruncDiv,
    Rem,
    Pow,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
    Is,
    IsNot,
    And,
    Or,
}

#[derive(Debug, Clone, Copy)]
pub enum Operation {
    Binop(Binop),
    Unop(Unop),
}

pub struct ArgumentList {
    positional: Vec<Expression>,
    keyword: Vec<(RcStr, Expression)>,
    variadic: Option<Box<Expression>>,
    table: Option<Box<Expression>>,
}

impl ArgumentList {
    pub fn new(
        positional: Vec<Expression>,
        keyword: Vec<(RcStr, Expression)>,
        variadic: Option<Expression>,
        table: Option<Expression>,
    ) -> ArgumentList {
        ArgumentList {
            positional,
            keyword,
            variadic: variadic.map(|e| e.into()),
            table: table.map(|e| e.into()),
        }
    }

    fn children_mut(&mut self) -> Vec<&mut Expression> {
        let ArgumentList {
            positional: args,
            keyword: kwargs,
            variadic,
            table: kwtable,
        } = self;
        join_mut_refs(vec![
            exprs2mutrefs(args),
            kwargs.iter_mut().map(|(_, e)| e).collect(),
            variadic.iter_mut().map(|b| &mut **b).collect(),
            kwtable.iter_mut().map(|b| &mut **b).collect(),
        ])
    }

    /// Tries to return a trivial version of the argument list, if there
    /// are no special non-positional argument types
    pub fn trivial(&self) -> Option<&Vec<Expression>> {
        if self.keyword.is_empty() && self.variadic.is_none() && self.table.is_none() {
            Some(&self.positional)
        } else {
            None
        }
    }

    pub fn positional(&self) -> &Vec<Expression> {
        &self.positional
    }

    pub fn keyword(&self) -> &Vec<(RcStr, Expression)> {
        &self.keyword
    }

    pub fn variadic(&self) -> &Option<Box<Expression>> {
        &self.variadic
    }

    pub fn table(&self) -> &Option<Box<Expression>> {
        &self.table
    }
}

pub enum ExpressionData {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    Symbol(Symbol),
    String(RcStr),
    MutableString(RcStr),
    Name(RcStr),
    Del(RcStr),
    Nonlocal(Vec<RcStr>),
    Parentheses(Box<Expression>),
    Block(Vec<Expression>),
    ListDisplay(Vec<Expression>),
    MapDisplay(Vec<(Expression, Expression)>),
    MutableListDisplay(Vec<Expression>),
    MutableMapDisplay(Vec<(Expression, Expression)>),
    Assign(Box<Expression>, Box<Expression>),
    AugAssign(Box<Expression>, Binop, Box<Expression>),
    AssignWithDoc(
        Box<Expression>, // Assign expression
        RcStr,           // variable name
        RcStr,           // doc
    ),
    If(
        Vec<(
            Expression, // condition
            Expression, // body
        )>,
        Option<Box<Expression>>, // other/else
    ),
    For(
        Box<Expression>, // target
        Box<Expression>, // iterable
        Box<Expression>, // body
    ),
    While(Box<Expression>, Box<Expression>),
    Unop(Unop, Box<Expression>),
    Binop(Binop, Box<Expression>, Box<Expression>),
    Attribute(Box<Expression>, RcStr),
    StaticAttribute(Box<Expression>, RcStr),
    Subscript(Box<Expression>, Box<Expression>),
    Slice(
        Box<Expression>,
        Option<Box<Expression>>,
        Option<Box<Expression>>,
    ),
    FunctionCall(Box<Expression>, ArgumentList),
    MethodCall(Box<Expression>, RcStr, ArgumentList),
    New(ArgumentList),
    FunctionDisplay(
        bool,                     // is generator?
        Option<RcStr>,            // name
        Vec<RcStr>,               // required params
        Vec<(RcStr, Expression)>, // optional params
        Option<RcStr>,            // variadic param
        Option<RcStr>,            // keywords param
        Option<RcStr>,            // doc
        Box<Expression>,          // body
    ),
    ClassDisplay(
        ClassKind,                // (trait, class or @class)
        RcStr,                    // short name
        Vec<Expression>,          // bases
        Option<RcStr>,            // docstring
        Option<Vec<Symbol>>,      // fields
        Vec<(RcStr, Expression)>, // methods
        Vec<(RcStr, Expression)>, // static methods
    ),
    ExceptionKindDisplay(
        RcStr,                   // short name
        Option<Box<Expression>>, // base
        Option<RcStr>,           // docstring
        Option<Vec<Symbol>>,     // fields
        Box<Expression>,         // message
    ),
    Import(RcStr),
    Yield(Box<Expression>),
    Return(Option<Box<Expression>>),

    // For debugging
    BreakPoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExpressionKind {
    Nil,
    Bool,
    Int,
    Float,
    Symbol,
    String,
    MutableString,
    Name,
    Del,
    Nonlocal,
    Parentheses,
    Block,
    ListDisplay,
    MapDisplay,
    MutableListDisplay,
    MutableMapDisplay,
    Assign,
    AugAssign,
    AssignWithDoc,
    If,
    For,
    While,
    Unop,
    Binop,
    Attribute,
    StaticAttribute,
    Subscript,
    Slice,
    FunctionCall,
    MethodCall,
    New,
    FunctionDisplay,
    ClassDisplay,
    ExceptionKindDisplay,
    Import,
    Yield,
    Return,
    BreakPoint,
}
