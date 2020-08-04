use super::*;

impl Globals {
    pub fn parse(&self, source: Rc<Source>) -> Result<ModuleDisplay> {
        let (tokens, posinfo) = match self.lexer.lex(source.data()) {
            Ok(r) => r,
            Err(error) => {
                let pos = error.offset();
                let lineno = error.lineno();
                return Err(Error::rt(
                    format!("{:?}", error).into(),
                    vec![Mark::new(source, pos, lineno)],
                ));
            }
        };
        self.parser.parse_tokens(source.clone(), tokens, posinfo)
    }
    pub fn repl_ready(&self, line: &str) -> bool {
        match self.lexer.lex(line) {
            Ok(_) => true,
            Err(error) => match error.kind() {
                LexErrorKind::UnmatchedOpeningSymbol | LexErrorKind::UnterminatedStringLiteral => {
                    false
                }

                // input is invalid, but the problem is not insufficient input
                _ => true,
            },
        }
    }
}
