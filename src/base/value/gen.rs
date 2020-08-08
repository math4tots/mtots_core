use super::*;

pub struct Generator {
    code: Rc<Code>,
    frame: Frame,
}

impl Generator {
    pub(crate) fn new(code: Rc<Code>, frame: Frame) -> Self {
        Self { code, frame }
    }
    pub fn resume(&mut self, globals: &mut Globals, arg: Value) -> ResumeResult {
        self.code.resume_frame(globals, &mut self.frame, arg)
    }
    pub fn unpack(&mut self, globals: &mut Globals) -> Result<Vec<Value>> {
        let mut ret = Vec::new();
        loop {
            match self.resume(globals, Value::Nil) {
                ResumeResult::Yield(value) => ret.push(value),
                ResumeResult::Return(_) => break,
                ResumeResult::Err(error) => return Err(error),
            }
        }
        Ok(ret)
    }
    pub fn iter<'a>(
        &'a mut self,
        globals: &'a mut Globals,
    ) -> impl Iterator<Item = Result<Value>> + 'a {
        geniter(self, globals)
    }
}

impl cmp::PartialEq for Generator {
    fn eq(&self, other: &Self) -> bool {
        self as *const _ == other as *const _
    }
}

impl cmp::PartialOrd for Generator {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        (self as *const Self as usize).partial_cmp(&(other as *const Self as usize))
    }
}

impl fmt::Debug for Generator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "<generator object {} at {:?}>",
            self.code.name(),
            self as *const _
        )
    }
}

pub struct NativeGenerator {
    name: Cow<'static, str>,
    body: Box<dyn FnMut(&mut Globals, Value) -> ResumeResult>,
}

impl NativeGenerator {
    pub fn new<B>(name: &'static str, body: B) -> Self
    where
        B: FnMut(&mut Globals, Value) -> ResumeResult + 'static,
    {
        Self {
            name: Cow::Borrowed(name),
            body: Box::new(body),
        }
    }
    pub fn new_with_dynamic_name<N, B>(name: N, body: B) -> Self
    where
        N: Into<String>,
        B: FnMut(&mut Globals, Value) -> ResumeResult + 'static,
    {
        Self {
            name: Cow::Owned(name.into()),
            body: Box::new(body),
        }
    }
    pub fn name(&self) -> &str {
        match &self.name {
            Cow::Borrowed(s) => s,
            Cow::Owned(s) => s,
        }
    }
    pub fn resume(&mut self, globals: &mut Globals, arg: Value) -> ResumeResult {
        (self.body)(globals, arg)
    }
    pub fn unpack(&mut self, globals: &mut Globals) -> Result<Vec<Value>> {
        let mut ret = Vec::new();
        loop {
            match self.resume(globals, Value::Nil) {
                ResumeResult::Yield(value) => ret.push(value),
                ResumeResult::Return(_) => break,
                ResumeResult::Err(error) => return Err(error),
            }
        }
        Ok(ret)
    }
    pub fn iter<'a>(
        &'a mut self,
        globals: &'a mut Globals,
    ) -> impl Iterator<Item = Result<Value>> + 'a {
        geniter(self, globals)
    }
}

impl cmp::PartialEq for NativeGenerator {
    fn eq(&self, other: &Self) -> bool {
        self as *const _ == other as *const _
    }
}

impl cmp::PartialOrd for NativeGenerator {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        (self as *const Self as usize).partial_cmp(&(other as *const Self as usize))
    }
}

impl fmt::Debug for NativeGenerator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "<native generator object {} at {:?}>",
            self.name, self as *const _
        )
    }
}

pub enum ResumeResult {
    Yield(Value),
    Return(Value),
    Err(Error),
}

trait BaseGenerator {
    fn resume(&mut self, globals: &mut Globals, arg: Value) -> ResumeResult;
}
fn geniter<'a, G: BaseGenerator>(
    gen: &'a mut G,
    globals: &'a mut Globals,
) -> impl Iterator<Item = Result<Value>> + 'a {
    struct Iter<'a, G: BaseGenerator> {
        gen: &'a mut G,
        gl: &'a mut Globals,
    }
    impl<'a, G: BaseGenerator> Iterator for Iter<'a, G> {
        type Item = Result<Value>;
        fn next(&mut self) -> Option<Result<Value>> {
            match self.gen.resume(self.gl, Value::Nil) {
                ResumeResult::Yield(value) => Some(Ok(value)),
                ResumeResult::Return(_) => None,
                ResumeResult::Err(error) => Some(Err(error)),
            }
        }
    }
    Iter { gen, gl: globals }
}
impl BaseGenerator for Generator {
    fn resume(&mut self, globals: &mut Globals, arg: Value) -> ResumeResult {
        self.resume(globals, arg)
    }
}
impl BaseGenerator for NativeGenerator {
    fn resume(&mut self, globals: &mut Globals, arg: Value) -> ResumeResult {
        self.resume(globals, arg)
    }
}
