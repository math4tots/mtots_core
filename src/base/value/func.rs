use super::*;

#[derive(Debug, Clone)]
pub struct ArgSpec {
    req: Vec<RcStr>,             // required parameters
    def: Vec<(RcStr, ConstVal)>, // default parameters
    var: Option<RcStr>,          // variadic parameter
    key: Option<RcStr>,          // keymap parameter
}

impl ArgSpec {
    pub fn new(
        req: Vec<RcStr>,
        def: Vec<(RcStr, ConstVal)>,
        var: Option<RcStr>,
        key: Option<RcStr>,
    ) -> Self {
        Self { req, def, var, key }
    }
    pub fn builder() -> ArgSpecBuilder {
        ArgSpecBuilder::default()
    }
    pub fn empty() -> Self {
        Self {
            req: vec![],
            def: vec![],
            var: None,
            key: None,
        }
    }
    pub(crate) fn add_self_parameter(&mut self) {
        self.req.insert(0, "self".into());
    }

    pub fn nparams(&self) -> usize {
        self.req.len()
            + self.def.len()
            + if self.var.is_some() { 1 } else { 0 }
            + if self.key.is_some() { 1 } else { 0 }
    }

    pub fn params(&self) -> Vec<RcStr> {
        let mut ret = self.req.clone();
        for (name, _) in &self.def {
            ret.push(name.clone());
        }
        if let Some(name) = &self.var {
            ret.push(name.clone());
        }
        if let Some(name) = &self.key {
            ret.push(name.clone());
        }
        ret
    }

    pub fn apply(
        &self,
        flatten_varargs: bool,
        mut args: Vec<Value>,
        mut kwargs: Option<HashMap<RcStr, Value>>,
    ) -> Result<(Vec<Value>, Option<HashMap<RcStr, Value>>)> {
        let lower = self.req.len();
        let upper = lower + self.def.len();
        if let Some(kwargs) = &mut kwargs {
            let mut iter = args.into_iter();
            let mut new_args = Vec::new();
            for name in &self.req {
                if let Some(val) = kwargs.remove(name) {
                    new_args.push(val);
                } else if let Some(val) = iter.next() {
                    new_args.push(val);
                } else {
                    return Err(rterr!("Missing argument for {:?} parameter", name));
                }
            }
            let mut exhausted = false;
            for (name, def) in &self.def {
                if let Some(val) = kwargs.remove(name) {
                    new_args.push(val);
                } else if exhausted {
                    new_args.push(def.clone().into());
                } else if let Some(val) = iter.next() {
                    new_args.push(val);
                } else {
                    exhausted = true;
                    new_args.push(def.clone().into());
                }
            }
            if !exhausted {
                if self.var.is_none() {
                    let extra_args: Vec<_> = iter.collect();
                    if extra_args.len() > 0 {
                        return Err(rterr!("Too many arguments ({:?})", extra_args));
                    }
                } else {
                    new_args.extend(iter);
                }
            }
            if self.key.is_none() {
                if kwargs.len() > 0 {
                    let keys: Vec<_> = kwargs.keys().collect();
                    return Err(rterr!("Unused keyword arguments: {:?}", keys));
                }
            }
            args = new_args;
        }
        let argc = args.len();
        if argc < lower || (argc > upper && self.var.is_none()) {
            return Err(if self.var.is_some() {
                rterr!("Expected at least {} args but got {}", lower, argc)
            } else if self.def.len() > 0 {
                rterr!("Expected at {} to {} args but got {}", lower, upper, argc)
            } else {
                rterr!("Expected {} args but got {}", lower, argc)
            });
        }
        if lower < upper && argc < upper {
            while args.len() < upper {
                let defval = self.def[args.len() - lower].1.clone();
                args.push(defval.into());
            }
        }
        if self.var.is_some() && flatten_varargs {
            let vec: Vec<_> = args.drain(upper..).collect();
            args.push(vec.into());
        }
        Ok((args, kwargs))
    }

    /// Like apply, but the kwargs map is converted to a Value and added to args
    /// if a kwargs parameter was specified
    pub fn apply_and_append_kwmap(
        &self,
        args: Vec<Value>,
        kwargs: Option<HashMap<RcStr, Value>>,
    ) -> Result<Vec<Value>> {
        let (mut args, kwargs) = self.apply(true, args, kwargs)?;
        if self.key.is_some() {
            if let Some(kwargs) = kwargs {
                args.push(kwargs.into());
            } else {
                args.push(Map::new().into())
            }
        }
        Ok(args)
    }

    pub fn to_value(&self) -> Value {
        Value::from(vec![
            Value::from(self.req.iter().map(Value::from).collect::<Vec<_>>()),
            Value::from(
                self.def
                    .iter()
                    .map(|(k, v)| Value::from(vec![Value::from(k), Value::from(v)]))
                    .collect::<Vec<_>>(),
            ),
            self.var.as_ref().map(Value::from).unwrap_or(Value::Nil),
            self.key.as_ref().map(Value::from).unwrap_or(Value::Nil),
        ])
    }
}

impl fmt::Display for ArgSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(")?;
        let mut first = true;
        for req in &self.req {
            if !first {
                write!(f, ", ")?;
            }
            first = false;

            write!(f, "{}", req)?;
        }
        for (name, val) in &self.def {
            if !first {
                write!(f, ", ")?;
            }
            first = false;

            write!(f, "{}={}", name, val)?;
        }
        if let Some(name) = &self.var {
            if !first {
                write!(f, ", ")?;
            }
            first = false;

            write!(f, "*{}", name)?;
        }
        if let Some(name) = &self.key {
            if !first {
                write!(f, ", ")?;
            }

            write!(f, "**{}", name)?;
        }
        write!(f, ")")
    }
}

impl From<()> for ArgSpec {
    fn from((): ()) -> Self {
        Self::empty()
    }
}

impl From<&[&str]> for ArgSpec {
    fn from(reqs: &[&str]) -> Self {
        Self {
            req: reqs.iter().map(RcStr::from).collect(),
            def: vec![],
            var: None,
            key: None,
        }
    }
}

macro_rules! from_arr_for_spec {
    ($n:tt) => {
        impl From<[&str; $n]> for ArgSpec {
            fn from(reqs: [&str; $n]) -> Self {
                let reqs: &[&str] = &reqs;
                reqs.into()
            }
        }
    };
}

from_arr_for_spec!(1);
from_arr_for_spec!(2);
from_arr_for_spec!(3);
from_arr_for_spec!(4);
from_arr_for_spec!(5);
from_arr_for_spec!(6);
from_arr_for_spec!(7);

impl From<ArgSpecBuilder> for ArgSpec {
    fn from(builder: ArgSpecBuilder) -> Self {
        builder.build()
    }
}

#[derive(Default)]
pub struct ArgSpecBuilder {
    req: Vec<RcStr>,
    def: Vec<(RcStr, ConstVal)>,
    var: Option<RcStr>,
    key: Option<RcStr>,
}

impl ArgSpecBuilder {
    pub fn build(self) -> ArgSpec {
        ArgSpec {
            req: self.req,
            def: self.def,
            var: self.var,
            key: self.key,
        }
    }
    pub fn req<S: Into<RcStr>>(mut self, name: S) -> Self {
        self.req.push(name.into());
        self
    }
    pub fn def<S: Into<RcStr>, V: Into<ConstVal>>(mut self, name: S, value: V) -> Self {
        self.def.push((name.into(), value.into()));
        self
    }
    pub fn var<S: Into<RcStr>>(mut self, optname: S) -> Self {
        self.var = Some(optname.into());
        self
    }
    pub fn key<S: Into<RcStr>>(mut self, keyname: S) -> Self {
        self.key = Some(keyname.into());
        self
    }
}

pub struct DocStr(Option<RcStr>);

impl DocStr {
    pub(crate) fn as_ref(&self) -> &Option<RcStr> {
        &self.0
    }
    pub(crate) fn get(self) -> Option<RcStr> {
        self.0
    }
}

impl From<()> for DocStr {
    fn from(_: ()) -> Self {
        Self(None)
    }
}

impl From<Option<RcStr>> for DocStr {
    fn from(doc: Option<RcStr>) -> Self {
        Self(doc)
    }
}

impl<T: Into<RcStr>> From<T> for DocStr {
    fn from(doc: T) -> Self {
        Self(Some(doc.into()))
    }
}

pub struct NativeFunction {
    name: RcStr,
    argspec: ArgSpec,
    doc: Option<RcStr>,
    body: Box<dyn Fn(&mut Globals, Vec<Value>, Option<HashMap<RcStr, Value>>) -> Result<Value>>,
}

impl NativeFunction {
    pub fn new<S, AS, D, B>(name: S, argspec: AS, doc: D, body: B) -> Self
    where
        S: Into<RcStr>,
        AS: Into<ArgSpec>,
        D: Into<DocStr>,
        B: Fn(&mut Globals, Vec<Value>, Option<HashMap<RcStr, Value>>) -> Result<Value> + 'static,
    {
        Self {
            name: name.into(),
            argspec: argspec.into(),
            doc: doc.into().0,
            body: Box::new(body),
        }
    }
    pub fn name(&self) -> &RcStr {
        &self.name
    }
    pub fn argspec(&self) -> &ArgSpec {
        &self.argspec
    }
    pub fn doc(&self) -> &Option<RcStr> {
        &self.doc
    }
    pub fn apply(
        &self,
        globals: &mut Globals,
        args: Vec<Value>,
        kwargs: Option<HashMap<RcStr, Value>>,
    ) -> Result<Value> {
        let (args, kwargs) = self.argspec.apply(false, args, kwargs)?;
        (self.body)(globals, args, kwargs)
    }
}

impl cmp::PartialEq for NativeFunction {
    fn eq(&self, other: &Self) -> bool {
        self as *const _ == other as *const _
    }
}

impl cmp::PartialOrd for NativeFunction {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        (self as *const Self as usize).partial_cmp(&(other as *const Self as usize))
    }
}

impl fmt::Debug for NativeFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<native function {}>", self.name())
    }
}

pub struct Function {
    argspec: Rc<ArgSpec>,
    code: Rc<Code>,
    bindings: Vec<Rc<RefCell<Value>>>,
    is_generator: bool,
}

impl cmp::PartialEq for Function {
    fn eq(&self, other: &Self) -> bool {
        self as *const _ == other as *const _
    }
}

impl cmp::PartialOrd for Function {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        (self as *const Self as usize).partial_cmp(&(other as *const Self as usize))
    }
}

impl fmt::Debug for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<function {}>", self.name())
    }
}

impl Function {
    pub fn new(
        argspec: Rc<ArgSpec>,
        code: Rc<Code>,
        bindings: Vec<Rc<RefCell<Value>>>,
        is_generator: bool,
    ) -> Self {
        assert_eq!(argspec.nparams(), code.params().len());
        Self {
            argspec,
            code,
            bindings,
            is_generator,
        }
    }
    pub fn name(&self) -> &RcStr {
        self.code.name()
    }
    pub fn doc(&self) -> &Option<RcStr> {
        self.code.doc()
    }
    pub fn argspec(&self) -> &ArgSpec {
        &self.argspec
    }
    pub fn is_generator(&self) -> bool {
        self.is_generator
    }
    pub fn code(&self) -> &Rc<Code> {
        &self.code
    }
    pub fn apply(
        &self,
        globals: &mut Globals,
        args: Vec<Value>,
        kwargs: Option<HashMap<RcStr, Value>>,
    ) -> Result<Value> {
        let args = self.argspec.apply_and_append_kwmap(args, kwargs)?;
        if self.is_generator {
            let frame = self.code.new_frame_with_args(self.bindings.clone(), args);
            Ok(Generator::new(self.code.clone(), frame).into())
        } else {
            self.code
                .apply_for_function(globals, self.bindings.clone(), args)
        }
    }
}
