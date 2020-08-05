use super::*;

pub struct Generator {
    code: Rc<Code>,
    frame: Frame,
}

impl Generator {
    pub(crate) fn new(code: Rc<Code>, frame: Frame) -> Self {
        Self {
            code,
            frame,
        }
    }
    pub fn resume(&mut self, globals: &mut Globals, arg: Value) -> ResumeResult {
        self.code.resume_frame(globals, &mut self.frame, arg)
    }
}

impl cmp::PartialEq for Generator {
    fn eq(&self, other: &Self) -> bool {
        self as *const _ == other as *const _
    }
}

impl fmt::Debug for Generator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<generator object {} at {:?}>", self.code.name(), self as *const _)
    }
}

pub enum ResumeResult {
    Yield(Value),
    Return(Value),
    Err(Error),
}
