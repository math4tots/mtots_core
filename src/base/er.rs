use crate::RcStr;
use std::fmt;
use std::fmt::Write;
use std::path::Path;
use std::rc::Rc;

pub type Result<T> = std::result::Result<T, Error>;

pub struct Source {
    name: RcStr,
    path: Option<Rc<Path>>,
    data: RcStr,
}

impl Source {
    pub fn new(name: RcStr, path: Option<Rc<Path>>, data: RcStr) -> Self {
        Self { name, path, data }
    }
    // the name of this source
    // this is the name you would use to import this module (e.g. 'a.foo.bar')
    pub fn name(&self) -> &RcStr {
        &self.name
    }
    // the path where this source was found, if available
    // Sometimes there's no good path for a source (e.g. REPL, one-of strings)
    pub fn path(&self) -> &Option<Rc<Path>> {
        &self.path
    }
    pub fn data(&self) -> &RcStr {
        &self.data
    }
}

impl fmt::Debug for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Source({})", self.name)
    }
}

#[derive(Debug, Clone)]
pub struct Mark {
    source: Rc<Source>,
    pos: usize,
    lineno: usize,
}

impl Mark {
    pub fn new(source: Rc<Source>, pos: usize, lineno: usize) -> Self {
        Self {
            source,
            pos,
            lineno,
        }
    }
    pub fn source(&self) -> &Rc<Source> {
        &self.source
    }
    pub fn format(&self) -> String {
        let mut ret = String::new();
        let out = &mut ret;
        writeln!(
            out,
            "in {}{} on line {}",
            self.source.name,
            if let Some(path) = self.source.path() {
                format!(" (file {:?})", path)
            } else {
                "".to_owned()
            },
            self.lineno()
        )
        .unwrap();
        let start = self.source.data[..self.pos]
            .rfind('\n')
            .map(|x| x + 1)
            .unwrap_or(0);
        let end = self.source.data[self.pos..]
            .find('\n')
            .map(|x| x + self.pos)
            .unwrap_or(self.source.data.len());
        writeln!(out, "{}", &self.source.data[start..end]).unwrap();
        for _ in start..self.pos {
            write!(out, " ").unwrap();
        }
        writeln!(out, "*").unwrap();
        ret
    }
    pub fn pos(&self) -> usize {
        self.pos
    }
    pub fn lineno(&self) -> usize {
        // TODO: use self.lineno... just make sure it's correct
        self.source.data[..self.pos].matches('\n').count() + 1
    }
}

pub struct ErrorData {
    type_: RcStr,
    message: RcStr,
    trace: Vec<Mark>,
}

pub struct Error(Rc<ErrorData>);

impl Error {
    pub fn new(type_: RcStr, message: RcStr, trace: Vec<Mark>) -> Self {
        Self(
            ErrorData {
                type_,
                message,
                trace,
            }
            .into(),
        )
    }
    pub fn rt(message: RcStr, trace: Vec<Mark>) -> Self {
        if message.is_empty() {
            panic!("Empty runtime error message");
        }
        Self::new("RuntimeError".into(), message, trace)
    }
    pub fn format(&self) -> String {
        format!("{}", self)
    }
    pub fn prepended(&self, mut trace: Vec<Mark>) -> Self {
        trace.extend(self.0.trace.clone());
        Self::new(self.0.type_.clone(), self.0.message.clone(), trace)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::new("IOError".into(), format!("{:?}", e).into(), vec![])
    }
}

impl From<std::fmt::Error> for Error {
    fn from(e: std::fmt::Error) -> Self {
        Self::rt(format!("{:?}", e).into(), vec![])
    }
}

impl From<std::convert::Infallible> for Error {
    fn from(_: std::convert::Infallible) -> Self {
        panic!("An Infallible Error has Ocurred")
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error({}, {})", self.0.type_, self.0.message)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "=================")?;
        writeln!(f, "== STACK TRACE ==")?;
        writeln!(f, "=================")?;
        for mark in &self.0.trace {
            write!(f, "{}", mark.format())?;
        }
        writeln!(f, "{}: {}", self.0.type_, self.0.message)
    }
}

impl std::error::Error for Error {}
