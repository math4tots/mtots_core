use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Token<'a> {
    Newline,
    NormalString(&'a str),
    RawString(&'a str),
    LineString(&'a str),
    Float(f64),
    Int(i64),
    Name(&'a str),
    Punctuator(Punctuator),
    EOF,
}

impl<'a> Token<'a> {
    pub fn kind(&self) -> TokenKind {
        match self {
            Token::Newline => TokenKind::Newline,
            Token::NormalString(_) => TokenKind::NormalString,
            Token::RawString(_) => TokenKind::RawString,
            Token::LineString(_) => TokenKind::LineString,
            Token::Float(_) => TokenKind::Float,
            Token::Int(_) => TokenKind::Int,
            Token::Name(_) => TokenKind::Name,
            Token::Punctuator(punctuator) => TokenKind::Punctuator(*punctuator),
            Token::EOF => TokenKind::EOF,
        }
    }

    pub fn normal_string(&self) -> Option<&'a str> {
        if let Token::NormalString(name) = self {
            Some(name)
        } else {
            None
        }
    }

    pub fn raw_string(&self) -> Option<&'a str> {
        if let Token::RawString(name) = self {
            Some(name)
        } else {
            None
        }
    }

    pub fn float(&self) -> Option<f64> {
        if let Token::Float(x) = self {
            Some(*x)
        } else {
            None
        }
    }

    pub fn int(&self) -> Option<i64> {
        if let Token::Int(x) = self {
            Some(*x)
        } else {
            None
        }
    }

    pub fn name(&self) -> Option<&'a str> {
        if let Token::Name(name) = self {
            Some(name)
        } else {
            None
        }
    }

    pub fn punctuator(&self) -> Option<Punctuator> {
        if let Token::Punctuator(punctuator) = self {
            Some(*punctuator)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Newline,
    NormalString,
    RawString,
    LineString,
    Float,
    Int,
    Name,
    EOF,
    Punctuator(Punctuator),
}

impl TokenKind {
    pub const LEN: usize = 8 + Punctuator::LIST.len();

    pub fn id(&self) -> usize {
        match self {
            TokenKind::Newline => 0,
            TokenKind::NormalString => 1,
            TokenKind::RawString => 2,
            TokenKind::LineString => 3,
            TokenKind::Float => 4,
            TokenKind::Int => 5,
            TokenKind::Name => 6,
            TokenKind::EOF => 7,
            TokenKind::Punctuator(punctuator) => 8 + (*punctuator as usize),
        }
    }

    pub fn str(&self) -> &'static str {
        match self {
            TokenKind::Newline => "Newline",
            TokenKind::NormalString => "NormalString",
            TokenKind::RawString => "RawString",
            TokenKind::LineString => "LineString",
            TokenKind::Float => "Float",
            TokenKind::Int => "Int",
            TokenKind::Name => "Name",
            TokenKind::EOF => "EOF",
            TokenKind::Punctuator(punctuator) => punctuator.str(),
        }
    }

    pub fn from_str(s: &str) -> Option<TokenKind> {
        Some(match s {
            "Newline" => TokenKind::Newline,
            "NormalString" => TokenKind::NormalString,
            "RawString" => TokenKind::RawString,
            "LineString" => TokenKind::LineString,
            "Float" => TokenKind::Float,
            "Int" => TokenKind::Int,
            "Name" => TokenKind::Name,
            "EOF" => TokenKind::EOF,
            _ => TokenKind::Punctuator(Punctuator::from_str(s)?),
        })
    }
}

macro_rules! define_punctuators {
    (
        $( $name:ident $value:expr, )*
    ) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Punctuator {
            $( $name , )*
        }

        impl Punctuator {

            pub const LIST: &'static [Punctuator] = &[ $( Punctuator::$name , )* ];

            pub fn str(&self) -> &'static str {
                match self {
                    $( Punctuator::$name => $value , )*
                }
            }

            pub fn from_str(s: &str) -> Option<Punctuator> {
                match s {
                    $( $value => Some(Punctuator::$name) ,)*
                    _ => None,
                }
            }
        }
    };
}

define_punctuators! {
    // keywords
    And "and",
    As "as",
    Break "break",
    Class "class",
    Continue "continue",
    Def "def",
    Del "del",
    Elif "elif",
    Else "else",
    Except "except",
    False "false",
    Final "final",
    For "for",
    From "from",
    If "if",
    Import "import",
    In "in",
    Is "is",
    Nil "nil",
    Nonlocal "nonlocal",
    Not "not",
    Or "or",
    Return "return",
    Static "static",
    Trait "trait",
    True "true",
    Try "try",
    While "while",
    Yield "yield",

    // operators and delimiters
    At "@",
    Dot ".",
    Scope "::",
    Colon ":",
    Semicolon ";",
    Comma ",",
    Plus "+",
    Minus "-",
    Rem "%",
    Star "*",
    Star2 "**",
    Slash "/",
    DoubleSlash "//",
    Lt "<",
    Le "<=",
    Gt ">",
    Ge ">=",
    Eq "=",
    Eq2 "==",
    Ne "!=",
    Exclamation "!",
    LParen "(",
    RParen ")",
    LBracket "[",
    RBracket "]",
    LBrace "{",
    RBrace "}",
}

impl fmt::Display for Punctuator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.str())
    }
}
