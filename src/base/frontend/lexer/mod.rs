mod trie;
use crate::Punctuator;
use crate::RcStr;
use crate::Token;

use trie::Trie;

#[derive(Debug, PartialEq)]
pub struct LexError {
    offset: usize, // byte offset from start of string where error occurred
    lineno: usize,
    kind: LexErrorKind,
}

#[derive(Debug, PartialEq)]
pub enum LexErrorKind {
    UnrecognizedToken(RcStr),
    UnterminatedStringLiteral,
    UnmatchedGroupingPunctuator,
    MismatchedGroupingPunctuator {
        open: Punctuator,
        open_offset: usize,
        open_lineno: usize,
    },
}

impl LexError {
    pub fn move_(self) -> (usize, usize, LexErrorKind) {
        (self.offset, self.lineno, self.kind)
    }
}

pub struct Lexer {
    punctuator_trie: Trie,
}

impl Lexer {
    pub fn new() -> Lexer {
        Lexer {
            punctuator_trie: Trie::build(Punctuator::LIST),
        }
    }

    pub fn lex<'a>(
        &self,
        mut s: &'a str,
    ) -> Result<(Vec<Token<'a>>, Vec<(usize, usize)>), LexError> {
        let mut tokens = Vec::new();
        let mut pos_info = Vec::new();
        let mut pos = 0; // current offset from the original string
        let mut lineno = 1;

        // Stack of grouping punctuators
        // for whether to skip newlines or not
        let mut gstack = Vec::new();

        loop {
            // skip spaces
            // dp = diff-pos, difference in position
            let dp: usize = s
                .chars()
                .take_while(|c| c.is_whitespace())
                .map(|c| c.len_utf8())
                .sum();
            let newlines = s[..dp].chars().filter(|c| *c == '\n').count();
            if newlines > 0 {
                lineno += newlines;
                if let None | Some((Punctuator::LBrace, _, _)) = gstack.last() {
                    add(&mut tokens, &mut pos_info, Token::Newline(newlines), pos, lineno);
                }
            }
            incr(&mut s, &mut pos, dp);

            if s.is_empty() {
                break;
            }
            let mut chars = s.chars();
            let c = chars.next().unwrap();
            let c2 = chars.next();

            // line/comment-style raw string literals
            if c == '#' {
                // Skip the '#'
                incr(&mut s, &mut pos, 1);

                // Allow the string to start with a ' ' to
                // make it look nicer
                if c2 == Some(' ') {
                    incr(&mut s, &mut pos, 1);
                }
                let len = s.find('\n').unwrap_or(s.len());
                add(
                    &mut tokens,
                    &mut pos_info,
                    Token::LineString(&s[..len]),
                    pos,
                    lineno,
                );
                incr(&mut s, &mut pos, len);
                continue;
            }

            // rust-style raw string literals
            if c == 'r' && c2 == Some('#') {
                // match the 'r##..#' part
                let mut diff = 'r'.len_utf8();
                let hash_start = diff;
                let hash_len: usize = s[diff..]
                    .chars()
                    .take_while(|c| *c == '#')
                    .map(|c| c.len_utf8())
                    .sum();
                diff += hash_len;
                let hash_end = diff;
                let hashes = &s[hash_start..hash_end];

                // match the quote
                let quote = match s[diff..].chars().next() {
                    Some('\'') => '\'',
                    Some('"') => '"',
                    _ => {
                        return Err(LexError {
                            offset: pos,
                            lineno: lineno,
                            kind: LexErrorKind::UnterminatedStringLiteral,
                        });
                    }
                };
                diff += quote.len_utf8();

                let string_start = diff;
                let terminator = format!("{}{}", quote, hashes);
                let string_len = match s[string_start..].find(&terminator) {
                    Some(i) => i,
                    None => {
                        return Err(LexError {
                            offset: pos,
                            lineno,
                            kind: LexErrorKind::UnterminatedStringLiteral,
                        });
                    }
                };
                let string_end = string_start + string_len;
                let string = &s[string_start..string_end];

                diff += string_len + terminator.len();

                let token = Token::RawString(string);
                add(&mut tokens, &mut pos_info, token, pos, lineno);
                incr(&mut s, &mut pos, diff);
                lineno += string.matches('\n').count();
                continue;
            }

            // string
            let at_str = c == '"' || c == '\'' || c == 'r' && (c2 == Some('"') || c2 == Some('\''));
            if at_str {
                // rs = s without raw indicator
                // rindlen = raw indicator len (the 'r' char)
                let (raw, rindlen, quote_char) = if c == 'r' {
                    (true, 'r'.len_utf8(), c2.unwrap())
                } else {
                    (false, 0, c)
                };
                let rs = &s[rindlen..];
                let quote_len = if rs.starts_with("'''") || rs.starts_with("\"\"\"") {
                    3
                } else {
                    1
                };

                let quote_bytelen = quote_char.len_utf8() * quote_len;

                let start_lineno = lineno;
                let mut esc = false;
                let mut matched_quote_len = 0;
                let mut done = false;
                let data_and_end_quotes_len: usize = rs[quote_bytelen..]
                    .chars()
                    .take_while(|c| {
                        if done {
                            false
                        } else {
                            if *c == '\n' {
                                lineno += 1;
                            }
                            if esc {
                                esc = false;
                                matched_quote_len = 0;
                            } else {
                                match *c {
                                    '\\' => {
                                        if !raw {
                                            esc = true;
                                        }
                                    }
                                    c if c == quote_char => {
                                        matched_quote_len += 1;
                                        if matched_quote_len == quote_len {
                                            done = true;
                                        }
                                    }
                                    _ => {
                                        matched_quote_len = 0;
                                    }
                                }
                            };
                            true
                        }
                    })
                    .map(|c| c.len_utf8())
                    .sum();

                if matched_quote_len != quote_len {
                    return Err(LexError {
                        offset: pos,
                        lineno: start_lineno,
                        kind: LexErrorKind::UnterminatedStringLiteral,
                    });
                }

                let text = &rs[quote_bytelen..data_and_end_quotes_len];
                let token = if raw {
                    Token::RawString(text)
                } else {
                    Token::NormalString(text)
                };
                add(&mut tokens, &mut pos_info, token, pos, lineno);
                incr(
                    &mut s,
                    &mut pos,
                    rindlen + quote_bytelen + data_and_end_quotes_len,
                );
                continue;
            }

            // number
            let num_dot = c == '.' && c2.map(|c| c.is_ascii_digit()).unwrap_or(false);
            if c.is_ascii_digit() || num_dot {
                let dp1: usize = s
                    .chars()
                    .take_while(|c| c.is_ascii_digit() || *c == '_')
                    .map(|c| c.len_utf8())
                    .sum();

                if s[dp1..].chars().next() == Some('.') {
                    // float
                    let dp1_with_dot = dp1 + '.'.len_utf8();
                    let dp2: usize = s[dp1_with_dot..]
                        .chars()
                        .take_while(|c| c.is_ascii_digit() || *c == '_')
                        .map(|c| c.len_utf8())
                        .sum();
                    let dp = dp1_with_dot + dp2;
                    let text = &s[..dp];
                    let token = Token::Float(filter_underscores(text).parse().unwrap());
                    add(&mut tokens, &mut pos_info, token, pos, lineno);
                    incr(&mut s, &mut pos, dp);
                } else if &s[..dp1] == "0" && s[dp1..].chars().next() == Some('x') {
                    // int (hex)
                    let dp1_with_x = dp1 + 'x'.len_utf8();
                    let dp2: usize = s[dp1_with_x..]
                        .chars()
                        .take_while(|c| c.is_ascii_hexdigit() || *c == '_')
                        .map(|c| c.len_utf8())
                        .sum();
                    let dp = dp1_with_x + dp2;
                    let text = &s[dp1_with_x..dp];
                    let token =
                        Token::Int(i64::from_str_radix(&filter_underscores(text), 16).unwrap());
                    add(&mut tokens, &mut pos_info, token, pos, lineno);
                    incr(&mut s, &mut pos, dp);
                } else {
                    // int
                    let text = &s[..dp1];
                    let token = Token::Int(filter_underscores(text).parse().unwrap());
                    add(&mut tokens, &mut pos_info, token, pos, lineno);
                    incr(&mut s, &mut pos, dp1);
                }
                continue;
            }

            // name or keyword
            if is_word(c) {
                let dp: usize = s
                    .chars()
                    .take_while(|c| is_word(*c))
                    .map(|c| c.len_utf8())
                    .sum();
                let text = &s[..dp];
                let token = if let Some(keyword) = self.punctuator_trie.find_exact(text) {
                    Token::Punctuator(keyword)
                } else {
                    Token::Name(text)
                };
                add(&mut tokens, &mut pos_info, token, pos, lineno);
                incr(&mut s, &mut pos, dp);
                continue;
            }

            // punctuator
            if let Some(punctuator) = self.punctuator_trie.find(s) {
                match punctuator {
                    Punctuator::LParen | Punctuator::LBracket | Punctuator::LBrace => {
                        gstack.push((punctuator, pos, lineno));
                    }
                    Punctuator::RParen | Punctuator::RBracket | Punctuator::RBrace => {
                        match gstack.pop() {
                            None => {
                                return Err(LexError {
                                    offset: pos,
                                    lineno,
                                    kind: LexErrorKind::UnmatchedGroupingPunctuator,
                                })
                            }
                            Some((open, open_pos, open_lineno)) => {
                                let matching = match punctuator {
                                    Punctuator::RParen => Punctuator::LParen,
                                    Punctuator::RBracket => Punctuator::LBracket,
                                    Punctuator::RBrace => Punctuator::LBrace,
                                    _ => panic!("{:?}", punctuator),
                                };
                                if matching != open {
                                    return Err(LexError {
                                        offset: pos,
                                        lineno,
                                        kind: LexErrorKind::MismatchedGroupingPunctuator {
                                            open,
                                            open_offset: open_pos,
                                            open_lineno,
                                        },
                                    });
                                }
                            }
                        }
                    }
                    _ => (),
                }
                let token = Token::Punctuator(punctuator);
                add(&mut tokens, &mut pos_info, token, pos, lineno);
                incr(&mut s, &mut pos, punctuator.str().len());
                continue;
            }

            // unrecognized
            let len = s
                .chars()
                .take_while(|c| !c.is_whitespace())
                .map(|c| c.len_utf8())
                .sum();
            let text = &s[..len];
            return Err(LexError {
                offset: pos,
                lineno,
                kind: LexErrorKind::UnrecognizedToken(text.into()),
            });
        }

        if let Some((_, open_pos, open_lineno)) = gstack.last() {
            return Err(LexError {
                offset: *open_pos,
                lineno: *open_lineno,
                kind: LexErrorKind::UnmatchedGroupingPunctuator,
            });
        }

        add(&mut tokens, &mut pos_info, Token::EOF, pos, lineno);
        Ok((tokens, pos_info))
    }
}

fn is_word(c: char) -> bool {
    c == '_' || c.is_alphabetic() || c.is_ascii_digit()
}

fn incr(s: &mut &str, pos: &mut usize, diff: usize) {
    *s = &(*s)[diff..];
    *pos += diff;
}

fn add<'a>(
    tokens: &mut Vec<Token<'a>>,
    posinfos: &mut Vec<(usize, usize)>,
    token: Token<'a>,
    offset: usize,
    lineno: usize,
) {
    tokens.push(token);
    posinfos.push((offset, lineno));
}

fn filter_underscores(s: &str) -> String {
    s.chars().filter(|c| *c != '_').collect()
}

#[cfg(test)]
mod tests {
    use super::LexError;
    use super::LexErrorKind;
    use super::Lexer;
    use super::Punctuator;
    use super::Token;

    fn lex<'a>(lexer: &Lexer, s: &'a str) -> Result<Vec<Token<'a>>, LexError> {
        let (tokens, postable) = lexer.lex(s)?;
        assert_eq!(tokens.len(), postable.len());
        Ok(tokens)
    }

    #[test]
    fn one_of_each() {
        let lexer = Lexer::new();

        assert_eq!(lex(&lexer, "\n").unwrap(), vec![Token::Newline(1), Token::EOF],);

        assert_eq!(
            lex(&lexer, "\n\n").unwrap(),
            vec![Token::Newline(2), Token::EOF],
        );

        assert_eq!(
            lex(&lexer, "'hi'").unwrap(),
            vec![Token::NormalString("hi"), Token::EOF],
        );

        assert_eq!(
            lex(&lexer, "\"hi\"").unwrap(),
            vec![Token::NormalString("hi"), Token::EOF],
        );

        assert_eq!(
            lex(&lexer, "r'hi'").unwrap(),
            vec![Token::RawString("hi"), Token::EOF],
        );

        assert_eq!(
            lex(&lexer, "r'''hey'''").unwrap(),
            vec![Token::RawString("hey"), Token::EOF],
        );

        assert_eq!(
            lex(&lexer, "\"\"\"hi\"\"\"").unwrap(),
            vec![Token::NormalString("hi"), Token::EOF],
        );

        assert_eq!(
            lex(&lexer, "r'\\'").unwrap(),
            vec![Token::RawString("\\"), Token::EOF],
        );

        assert_eq!(
            lex(&lexer, "'\\'"),
            Err(LexError {
                offset: 0,
                lineno: 1,
                kind: LexErrorKind::UnterminatedStringLiteral,
            })
        );

        assert_eq!(
            lex(&lexer, "5.8").unwrap(),
            vec![Token::Float(5.8), Token::EOF],
        );

        assert_eq!(lex(&lexer, "12").unwrap(), vec![Token::Int(12), Token::EOF]);

        assert_eq!(
            lex(&lexer, "0x1A2").unwrap(),
            vec![Token::Int(418), Token::EOF],
        );

        assert_eq!(
            lex(&lexer, "hello").unwrap(),
            vec![Token::Name("hello"), Token::EOF,],
        );

        assert_eq!(
            lex(&lexer, "def").unwrap(),
            vec![Token::Punctuator(Punctuator::Def), Token::EOF],
        );

        assert_eq!(
            lex(&lexer, "()").unwrap(),
            vec![
                Token::Punctuator(Punctuator::LParen),
                Token::Punctuator(Punctuator::RParen),
                Token::EOF,
            ],
        );

        assert_eq!(
            lex(&lexer, "==").unwrap(),
            vec![Token::Punctuator(Punctuator::Eq2), Token::EOF],
        );

        assert_eq!(
            lex(&lexer, "=").unwrap(),
            vec![Token::Punctuator(Punctuator::Eq), Token::EOF],
        );
    }

    #[test]
    fn grouping_punctuators() {
        let lexer = Lexer::new();
        assert_eq!(
            lex(&lexer, "("),
            Err(LexError {
                offset: 0,
                lineno: 1,
                kind: LexErrorKind::UnmatchedGroupingPunctuator,
            }),
        );
        assert_eq!(
            lex(&lexer, ")"),
            Err(LexError {
                offset: 0,
                lineno: 1,
                kind: LexErrorKind::UnmatchedGroupingPunctuator,
            }),
        );
        assert_eq!(
            lex(&lexer, "(]"),
            Err(LexError {
                offset: 1,
                lineno: 1,
                kind: LexErrorKind::MismatchedGroupingPunctuator {
                    open: Punctuator::LParen,
                    open_offset: 0,
                    open_lineno: 1,
                },
            }),
        );
        assert_eq!(
            lex(&lexer, "(\n)"),
            Ok(vec![
                Token::Punctuator(Punctuator::LParen),
                Token::Punctuator(Punctuator::RParen),
                Token::EOF,
            ]),
        );
        assert_eq!(
            lex(&lexer, "[\n]"),
            Ok(vec![
                Token::Punctuator(Punctuator::LBracket),
                Token::Punctuator(Punctuator::RBracket),
                Token::EOF,
            ]),
        );
        assert_eq!(
            lex(&lexer, "{\n}"),
            Ok(vec![
                Token::Punctuator(Punctuator::LBrace),
                Token::Newline(1),
                Token::Punctuator(Punctuator::RBrace),
                Token::EOF,
            ]),
        );
    }

    #[test]
    fn mixed() {
        let lexer = Lexer::new();
        let tokens = lex(
            &lexer,
            r##"
            def foo() {
                1 + 2
            }
        "##,
        )
        .unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Newline(1),
                Token::Punctuator(Punctuator::Def),
                Token::Name("foo"),
                Token::Punctuator(Punctuator::LParen),
                Token::Punctuator(Punctuator::RParen),
                Token::Punctuator(Punctuator::LBrace),
                Token::Newline(1),
                Token::Int(1),
                Token::Punctuator(Punctuator::Plus),
                Token::Int(2),
                Token::Newline(1),
                Token::Punctuator(Punctuator::RBrace),
                Token::Newline(1),
                Token::EOF,
            ]
        );
    }

    #[test]
    fn error_unrecognized() {
        let lexer = Lexer::new();
        assert_eq!(
            lex(&lexer, "asdf $%^"),
            Err(LexError {
                offset: 5,
                lineno: 1,
                kind: LexErrorKind::UnrecognizedToken("$%^".into()),
            }),
        );
    }
}
