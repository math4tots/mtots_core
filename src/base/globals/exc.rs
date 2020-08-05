use crate::ParameterInfo;
use crate::RcStr;
use crate::Symbol;
use crate::Value;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::rc::Rc;

pub struct ExceptionRegistry {
    by_id: Vec<Rc<ExceptionKind>>,
    children: Vec<Vec<usize>>,
}

impl ExceptionRegistry {
    fn add0(
        &mut self,
        base: Option<Rc<ExceptionKind>>,
        name: RcStr,
        message: RcStr,
        fields: Option<Vec<RcStr>>,
    ) -> Rc<ExceptionKind> {
        let id = self.by_id.len();
        assert_eq!(self.children.len(), id);
        self.children.push(Vec::new());
        let mut ancestry: HashSet<usize> = vec![id].into_iter().collect();
        if let Some(base) = base {
            self.children[base.id].push(id);
            assert!(
                base.fields.is_none(),
                "Exceptions with fields cannot be inherited ({}, {}, {:?})",
                base.name,
                name,
                base.fields,
            );
            ancestry.extend(base.ancestry.clone());
        }
        let fields_as_parameter_info = match &fields {
            Some(fields) => {
                let mut names_as_symbols = Vec::new();
                for field in fields {
                    names_as_symbols.push(Symbol::from(field));
                }
                ParameterInfo::new(names_as_symbols, vec![], None, None)
            }
            None => ParameterInfo::builder().optional("optarg", Value::Uninitialized).build(),
        };
        let rc = Rc::new(ExceptionKind {
            id,
            ancestry,
            name,
            message,
            fields,
            fields_as_parameter_info,
        });
        self.by_id.push(rc.clone());
        rc
    }

    pub fn add(
        &mut self,
        base: Rc<ExceptionKind>,
        name: RcStr,
        message: RcStr,
        fields: Option<Vec<RcStr>>,
    ) -> Rc<ExceptionKind> {
        self.add0(Some(base), name, message, fields)
    }

    fn add_without_base(&mut self, name: RcStr, message: RcStr) -> Rc<ExceptionKind> {
        self.add0(None, name, message, None)
    }
}

pub struct ExceptionKind {
    id: usize,
    ancestry: HashSet<usize>,
    name: RcStr,
    message: RcStr,
    fields: Option<Vec<RcStr>>,
    fields_as_parameter_info: ParameterInfo,
}

impl ExceptionKind {
    pub fn id(&self) -> usize {
        self.id
    }
    pub fn name(&self) -> &RcStr {
        &self.name
    }
    pub fn fields(&self) -> &Option<Vec<RcStr>> {
        &self.fields
    }
    pub fn fields_as_parameter_info(&self) -> &ParameterInfo {
        &self.fields_as_parameter_info
    }
}

impl std::cmp::PartialEq for ExceptionKind {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl std::cmp::Eq for ExceptionKind {}

impl fmt::Debug for ExceptionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<ExceptionKind {}>", self.name)
    }
}

pub struct Exception {
    kind: Rc<ExceptionKind>,
    args: Vec<Value>,
}

impl fmt::Debug for Exception {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{:?}", self.kind.name, self.args)
    }
}

impl fmt::Display for Exception {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use fmt::Write;
        write!(f, "{}", self.kind.name)?;
        if self.kind.fields.is_none() && !self.args.is_empty() {
            assert_eq!(self.args.len(), 1);
            write!(f, ": {}", self.args[0])?;
        } else if self.kind.message.is_empty() {
            write!(f, ": {:?}", self.args)?;
        } else {
            write!(f, ": ")?;
            let map: HashMap<RcStr, Value> = self
                .kind
                .fields
                .as_ref()
                .unwrap_or(&vec![])
                .clone()
                .into_iter()
                .zip(self.args.clone())
                .collect();
            enum Mode {
                Normal,
                Name(String),
            }
            let mut mode = Mode::Normal;
            for c in self.kind.message.chars() {
                match &mut mode {
                    Mode::Normal => match c {
                        '{' => mode = Mode::Name(String::new()),
                        '}' => panic!(
                            "Unexpected close brace in format string {}",
                            self.kind.message
                        ),
                        _ => f.write_char(c)?,
                    },
                    Mode::Name(name) => match c {
                        '}' => {
                            let name: &str = name;
                            let name: RcStr = name.into();
                            write!(f, "{}", map[&name])?;
                            mode = Mode::Normal;
                        }
                        _ => name.push(c),
                    },
                }
            }
        }
        Ok(())
    }
}

impl Exception {
    pub fn new(kind: Rc<ExceptionKind>, args: Vec<Value>) -> Exception {
        if let Some(fields) = &kind.fields {
            assert_eq!(fields.len(), args.len());
        } else {
            assert!(args.is_empty() || args.len() == 1);
        }
        Exception { kind, args }
    }

    pub fn matches(&self, kind: &ExceptionKind) -> bool {
        self.kind.ancestry.contains(&kind.id)
    }

    pub fn kind(&self) -> &Rc<ExceptionKind> {
        &self.kind
    }

    pub fn args(&self) -> &Vec<Value> {
        &self.args
    }
}

#[allow(non_snake_case)]
pub struct BuiltinExceptions {
    pub BaseException: Rc<ExceptionKind>,
    pub EscapeToTrampoline: Rc<ExceptionKind>,
    pub Exception: Rc<ExceptionKind>,
    pub RuntimeError: Rc<ExceptionKind>,
    pub NameError: Rc<ExceptionKind>,
    pub TypeError: Rc<ExceptionKind>,
    pub ExpectedTypeError: Rc<ExceptionKind>,
    pub ValueKindError: Rc<ExceptionKind>,
    pub OperandTypeError: Rc<ExceptionKind>,
    pub AttributeError: Rc<ExceptionKind>,
    pub InstanceAttributeError: Rc<ExceptionKind>,
    pub StaticAttributeError: Rc<ExceptionKind>,
    pub KeyError: Rc<ExceptionKind>,
    pub PopFromEmptyCollection: Rc<ExceptionKind>,
    pub AssertionError: Rc<ExceptionKind>,
    pub HashError: Rc<ExceptionKind>,
    pub UnpackError: Rc<ExceptionKind>,
    pub OSError: Rc<ExceptionKind>,
}

impl BuiltinExceptions {
    pub fn for_builtins(&self) -> Vec<&Rc<ExceptionKind>> {
        vec![
            &self.BaseException,
            &self.Exception,
            &self.RuntimeError,
            &self.NameError,
            &self.TypeError,
            &self.AttributeError,
            &self.KeyError,
            &self.OSError,
        ]
    }
}

#[allow(non_snake_case)]
pub(super) fn new() -> (ExceptionRegistry, BuiltinExceptions) {
    let mut registry = ExceptionRegistry {
        by_id: Vec::new(),
        children: Vec::new(),
    };

    let BaseException = registry.add_without_base("BaseException".into(), "".into());
    let EscapeToTrampoline = registry.add(
        BaseException.clone(),
        "EscapeToTrampoline".into(),
        "Escape to trampoline requested".into(),
        None,
    );
    let Exception = registry.add(BaseException.clone(), "Exception".into(), "".into(), None);
    let RuntimeError = registry.add(
        Exception.clone(),
        "RuntimeError".into(),
        "General error encountered by the interpreter".into(),
        None,
    );
    let NameError = registry.add(
        RuntimeError.clone(),
        "NameError".into(),
        "\"{name}\" not found".into(),
        Some(vec!["name".into()]),
    );
    let TypeError = registry.add(RuntimeError.clone(), "TypeError".into(), "".into(), None);
    let ExpectedTypeError = registry.add(
        TypeError.clone(),
        "ExpectedTypeError".into(),
        "Expected {expected} but got {got}".into(),
        Some(vec!["expected".into(), "got".into()]),
    );
    let ValueKindError = registry.add(
        TypeError.clone(),
        "ValueKindError".into(),
        "Expected {expected} but got {got}".into(),
        Some(vec!["expected".into(), "got".into()]),
    );
    let OperandTypeError = registry.add(
        TypeError.clone(),
        "OperandTypeError".into(),
        "{operation} not supported for {operands}".into(),
        Some(vec!["operation".into(), "operands".into()]),
    );
    let AttributeError = registry.add(
        RuntimeError.clone(),
        "AttributeError".into(),
        "".into(),
        None,
    );
    let InstanceAttributeError = registry.add(
        AttributeError.clone(),
        "InstanceAttributeError".into(),
        "Attribute {name} is not available for {class}".into(),
        Some(vec!["name".into(), "class".into()]),
    );
    let StaticAttributeError = registry.add(
        AttributeError.clone(),
        "StaticAttributeError".into(),
        "Attribute \"{name}\" is not available for {item}".into(),
        Some(vec!["name".into(), "item".into()]),
    );
    let KeyError = registry.add(RuntimeError.clone(), "KeyError".into(), "".into(), None);
    let PopFromEmptyCollection = registry.add(
        RuntimeError.clone(),
        "PopFromEmptyCollection".into(),
        "Tried to pop from an empty collection".into(),
        None,
    );
    let AssertionError = registry.add(
        RuntimeError.clone(),
        "AssertionError".into(),
        "{message}".into(),
        Some(vec!["message".into()]),
    );
    let HashError = registry.add(
        RuntimeError.clone(),
        "HashError".into(),
        "Value of kind {kind} cannot be hashed".into(),
        Some(vec!["kind".into()]),
    );
    let UnpackError = registry.add(
        RuntimeError.clone(),
        "UnpackError".into(),
        "Expected {expected} values but got {got}".into(),
        Some(vec!["expected".into(), "got".into()]),
    );
    let OSError = registry.add(RuntimeError.clone(), "OSError".into(), "".into(), None);

    (
        registry,
        BuiltinExceptions {
            BaseException,
            EscapeToTrampoline,
            Exception,
            RuntimeError,
            NameError,
            TypeError,
            ExpectedTypeError,
            ValueKindError,
            OperandTypeError,
            AttributeError,
            InstanceAttributeError,
            StaticAttributeError,
            KeyError,
            PopFromEmptyCollection,
            AssertionError,
            HashError,
            UnpackError,
            OSError,
        },
    )
}
