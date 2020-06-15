#[derive(Debug, Clone, PartialEq)]
pub enum Token<'a> {
    Name(&'a str),
    Number(f64),
    RawString(&'a str),
    String(String),
    EOF,

    // Single character symbols
    Newline,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Dollar,
    Comma,
    Semicolon,
    Plus,
    Minus,
    Star,
    Slash,
    Slash2,
    Rem,
    Eq,
    Bar,
}

impl<'a> Token<'a> {
    pub fn name(&self) -> Option<&str> {
        if let Token::Name(s) = self {
            Some(s)
        } else {
            None
        }
    }
    #[allow(dead_code)]
    pub fn number(&self) -> Option<f64> {
        if let Token::Number(x) = self {
            Some(*x)
        } else {
            None
        }
    }
    #[allow(dead_code)]
    pub fn raw_string(&self) -> Option<&str> {
        if let Token::RawString(x) = self {
            Some(x)
        } else {
            None
        }
    }
    #[allow(dead_code)]
    pub fn string(self) -> Option<String> {
        if let Token::String(x) = self {
            Some(x)
        } else {
            None
        }
    }
}

pub fn lex(s: &str) -> Result<Vec<Token>, String> {
    let mut ret = Vec::<Token>::new();
    let mut state = State::Neutral;
    let mut last_ig_ws = 0;
    let mut pstack = ParenStack::new();
    let mut chars = Chars::new(s);
    while let Some(c) = chars.next() {
        let i = chars.index - c.len_utf8();
        match state {
            State::Neutral => {
                if (c == '\n' && pstack.ignore_newline()) || c.is_whitespace() {
                    // skip whitespace
                    // We also keep track of the last ignored whitespace
                    // to figure out when tokens should be combined
                    last_ig_ws = i;
                    state = State::Neutral;
                } else if c.is_ascii_digit() {
                    state = State::Digits(i);
                } else if c == '_' || c.is_alphanumeric() {
                    state = State::Name(i);
                } else if c == '"' || c == '\'' {
                    if let Some(Token::Name("r")) = ret.last() {
                        ret.pop().unwrap();
                        state = State::RawString(c, i + c.len_utf8());
                    } else {
                        state = State::String(c, String::new());
                    }
                } else {
                    let tok = match c {
                        '\0' => Some(Token::EOF),
                        '\n' => Some(Token::Newline),
                        '(' => Some(Token::LParen),
                        ')' => Some(Token::RParen),
                        '[' => Some(Token::LBracket),
                        ']' => Some(Token::RBracket),
                        '{' => Some(Token::LBrace),
                        '}' => Some(Token::RBrace),
                        '$' => Some(Token::Dollar),
                        ',' => Some(Token::Comma),
                        ';' => Some(Token::Semicolon),
                        '+' => Some(Token::Plus),
                        '-' => Some(Token::Minus),
                        '*' => Some(Token::Star),
                        '/' => Some(if ret.last() == Some(&Token::Slash) && last_ig_ws < i - 1 {
                            ret.pop().unwrap();
                            Token::Slash2
                        } else {
                            Token::Slash
                        }),
                        '%' => Some(Token::Rem),
                        '=' => Some(Token::Eq),
                        '|' => Some(Token::Bar),
                        _ => None,
                    };
                    if let Some(tok) = tok {
                        match tok {
                            Token::LParen | Token::LBracket => pstack.push(true),
                            Token::LBrace => pstack.push(false),
                            Token::RParen | Token::RBracket | Token::RBrace => pstack.pop()?,
                            _ => (),
                        }
                        ret.push(tok);
                        state = State::Neutral;
                    } else {
                        return Err(format!("Unrecognized token: {}", c));
                    }
                }
            }
            State::Digits(start) => {
                if c.is_ascii_digit() {
                    state = State::Digits(start);
                } else if c == '.' {
                    state = State::Number(start);
                } else {
                    chars.put_back(c);
                    state = State::Number(start);
                }
            }
            State::Number(start) => {
                if c.is_ascii_digit() {
                    state = State::Number(start);
                } else {
                    let n: f64 = s[start..i].parse().unwrap();
                    ret.push(Token::Number(n));
                    chars.put_back(c);
                    state = State::Neutral;
                }
            }
            State::Name(start) => {
                if c == '_' || c.is_alphanumeric() {
                    state = State::Name(start);
                } else {
                    ret.push(Token::Name(&s[start..i]));
                    chars.put_back(c);
                    state = State::Neutral;
                }
            }
            State::RawString(q, start) => {
                if c == q {
                    ret.push(Token::RawString(&s[start..i]));
                    state = State::Neutral;
                } else {
                    state = State::RawString(q, start);
                }
            }
            State::String(q, mut string) => {
                if c == q {
                    ret.push(Token::String(string));
                    state = State::Neutral;
                } else if c == '\\' {
                    state = State::StringEscaped(q, string);
                } else {
                    string.push(c);
                    state = State::String(q, string);
                }
            }
            State::StringEscaped(q, mut string) => {
                let s = match c {
                    '\\' => "\\",
                    '\'' => "\'",
                    '\"' => "\"",
                    't' => "\t",
                    'n' => "\n",
                    'r' => "\r",
                    _ => return Err(format!("Invalid string escape ({})", c)),
                };
                string.push_str(s);
                state = State::String(q, string);
            }
        }
    }
    if let State::Neutral = &state {
        Ok(ret)
    } else {
        Err(format!("Expected more input: {:?}", state))
    }
}

#[derive(Debug)]
enum State {
    Neutral,
    Digits(usize),
    Number(usize),
    Name(usize),
    RawString(char, usize),
    String(char, String),
    StringEscaped(char, String),
}

struct ParenStack {
    stack: Vec<bool>,
}

impl ParenStack {
    pub fn new() -> ParenStack {
        ParenStack { stack: Vec::new() }
    }
    pub fn push(&mut self, ignore_newline: bool) {
        self.stack.push(ignore_newline)
    }
    pub fn pop(&mut self) -> Result<(), String> {
        match self.stack.pop() {
            Some(_) => Ok(()),
            None => Err(format!("Mismatched grouping symbols")),
        }
    }
    pub fn ignore_newline(&self) -> bool {
        self.stack.last().cloned().unwrap_or(false)
    }
}

struct Chars<'a> {
    index: usize,
    peek: Option<char>,
    chars: std::iter::Chain<std::str::Chars<'a>, std::vec::IntoIter<char>>,
}

impl<'a> Chars<'a> {
    fn new(s: &str) -> Chars {
        Chars {
            index: 0,
            peek: None,
            chars: s.chars().chain(vec!['\0']),
        }
    }
    fn next(&mut self) -> Option<char> {
        let ch = if let Some(ch) = std::mem::replace(&mut self.peek, None) {
            Some(ch)
        } else {
            self.chars.next()
        };
        if let Some(ch) = ch {
            self.index += ch.len_utf8();
        }
        ch
    }
    fn put_back(&mut self, c: char) {
        assert!(self.peek.is_none());
        self.peek = Some(c);
        self.index -= c.len_utf8();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample() {
        let toks = lex("Hello, world").unwrap();
        assert_eq!(
            toks,
            vec![
                Token::Name("Hello"),
                Token::Comma,
                Token::Name("world"),
                Token::EOF,
            ]
        );
    }

    #[test]
    fn float() {
        let toks = lex("44").unwrap();
        assert_eq!(toks, vec![Token::Number(44.0), Token::EOF,]);
        let toks = lex("56.78").unwrap();
        assert_eq!(toks, vec![Token::Number(56.78), Token::EOF,]);
    }

    #[test]
    fn string() {
        let toks = lex("'hello'").unwrap();
        assert_eq!(toks, vec![Token::String("hello".to_owned()), Token::EOF,]);
        let toks = lex("\"with double quotes\"").unwrap();
        assert_eq!(
            toks,
            vec![Token::String("with double quotes".to_owned()), Token::EOF,]
        );
        let toks = lex("\"with\nescapes\"").unwrap();
        assert_eq!(
            toks,
            vec![Token::String("with\nescapes".to_owned()), Token::EOF,]
        );
        let toks = lex("r'some raw string'").unwrap();
        assert_eq!(toks, vec![Token::RawString("some raw string"), Token::EOF,]);
    }
}
