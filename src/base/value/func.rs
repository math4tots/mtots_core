use super::*;

#[derive(Debug, Clone)]
pub struct ArgSpec {
    req: Vec<RcStr>,             // required parameters
    def: Vec<(RcStr, ConstVal)>, // default parameters
    var: Option<RcStr>,          // variadic parameter
}

impl ArgSpec {
    pub fn new(req: Vec<RcStr>, def: Vec<(RcStr, ConstVal)>, var: Option<RcStr>) -> Self {
        Self { req, def, var }
    }
    pub fn builder() -> ArgSpecBuilder {
        ArgSpecBuilder::default()
    }
    pub fn empty() -> Self {
        Self {
            req: vec![],
            def: vec![],
            var: None,
        }
    }

    pub fn nparams(&self) -> usize {
        self.req.len() + self.def.len() + if self.var.is_some() { 1 } else { 0 }
    }

    pub fn params(&self) -> Vec<RcStr> {
        let mut ret = self.req.clone();
        for (name, _) in &self.def {
            ret.push(name.clone());
        }
        if let Some(name) = &self.var {
            ret.push(name.clone());
        }
        ret
    }

    pub fn apply(
        &self,
        mut args: Vec<Value>,
        kwargs: Option<HashMap<RcStr, Value>>,
    ) -> Result<Vec<Value>> {
        let lower = self.req.len();
        let upper = lower + self.def.len();
        if let Some(mut kwargs) = kwargs {
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
                    return Err(rterr!("Too many arguments"));
                } else {
                    new_args.extend(iter);
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
        if self.var.is_some() {
            let vec: Vec<_> = args.drain(upper..).collect();
            args.push(vec.into());
        }
        Ok(args)
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

#[derive(Default)]
pub struct ArgSpecBuilder {
    req: Vec<RcStr>,
    def: Vec<(RcStr, ConstVal)>,
    var: Option<RcStr>,
}

impl ArgSpecBuilder {
    pub fn build(self) -> ArgSpec {
        ArgSpec {
            req: self.req,
            def: self.def,
            var: self.var,
        }
    }
    pub fn req<S: Into<RcStr>>(&mut self, name: S) -> &mut Self {
        self.req.push(name.into());
        self
    }
    pub fn def<S: Into<RcStr>, V: Into<ConstVal>>(&mut self, name: S, value: V) -> &mut Self {
        self.def.push((name.into(), value.into()));
        self
    }
    pub fn var<S: Into<Option<RcStr>>>(&mut self, optname: S) -> &mut Self {
        self.var = optname.into();
        self
    }
}

pub type NativeFunctionBody = fn(&mut Globals, args: Vec<Value>) -> Result<Value>;

pub struct NativeFunction {
    name: RcStr,
    argspec: ArgSpec,
    body: NativeFunctionBody,
}

impl NativeFunction {
    pub fn new<S: Into<RcStr>, AS: Into<ArgSpec>>(
        name: S,
        argspec: AS,
        body: NativeFunctionBody,
    ) -> Self {
        Self {
            name: name.into(),
            argspec: argspec.into(),
            body,
        }
    }
    pub fn name(&self) -> &RcStr {
        &self.name
    }
    pub fn apply(
        &self,
        globals: &mut Globals,
        args: Vec<Value>,
        kwargs: Option<HashMap<RcStr, Value>>,
    ) -> Result<Value> {
        let args = self.argspec.apply(args, kwargs)?;
        (self.body)(globals, args)
    }
}

impl cmp::PartialEq for NativeFunction {
    fn eq(&self, other: &Self) -> bool {
        self.body as *const std::ffi::c_void == other.body as *const std::ffi::c_void
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
        if argspec.nparams() != code.params().len() {
            println!("code.name = {}", code.name());
        }
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
    pub fn apply(
        &self,
        globals: &mut Globals,
        args: Vec<Value>,
        kwargs: Option<HashMap<RcStr, Value>>,
    ) -> Result<Value> {
        let args = self.argspec.apply(args, kwargs)?;
        if self.is_generator {
            let frame = self.code.new_frame(self.bindings.clone());
            Ok(Generator::new(self.code.clone(), frame).into())
        } else {
            self.code
                .apply_for_function(globals, self.bindings.clone(), args)
        }
    }
}
