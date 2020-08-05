use super::*;

pub struct Frame {
    stack: Vec<Value>,
    locals: Vec<Value>,
    upvals: Vec<Rc<RefCell<Value>>>,
    pc: usize,
}

impl Frame {
    pub(super) fn new(
        nlocals: usize,
        bindings: Vec<Rc<RefCell<Value>>>,
        nownedvars: usize,
    ) -> Self {
        Self {
            stack: vec![],
            locals: {
                let mut vec = Vec::new();
                vec.resize_with(nlocals, || Value::Invalid);
                vec
            },
            upvals: {
                let mut vec = bindings;
                vec.resize_with(vec.len() + nownedvars, || {
                    Rc::new(RefCell::new(Value::Invalid))
                });
                vec
            },
            pc: 0,
        }
    }
    #[inline(always)]
    pub(super) fn len(&mut self) -> usize {
        self.stack.len()
    }
    #[inline(always)]
    pub(super) fn swap01(&mut self) {
        let len = self.stack.len();
        self.stack.swap(len - 2, len - 1);
    }
    #[inline(always)]
    pub(super) fn push(&mut self, value: Value) {
        self.stack.push(value);
    }
    #[inline(always)]
    pub(super) fn pop(&mut self) -> Value {
        self.stack.pop().unwrap()
    }
    #[inline(always)]
    pub(super) fn peek(&self) -> &Value {
        self.stack.last().unwrap()
    }
    #[inline(always)]
    pub(super) fn popn(&mut self, n: usize) -> Vec<Value> {
        let len = self.stack.len();
        self.stack.drain(len - n..).collect()
    }
    #[inline(always)]
    pub(super) fn pushn<I: IntoIterator<Item = Value>>(&mut self, vec: I) {
        self.stack.extend(vec);
    }
    #[inline(always)]
    pub(super) fn setvar(&mut self, var: &Variable, value: Value) {
        match var.type_() {
            VariableType::Local => self.locals[var.slot()] = value,
            VariableType::Upval => {
                self.upvals[var.slot()].replace(value);
            }
        }
    }
    #[inline(always)]
    pub(super) fn getcell(&self, slot: usize) -> Rc<RefCell<Value>> {
        self.upvals[slot].clone()
    }
    #[inline(always)]
    pub(super) fn getcells(&self) -> &Vec<Rc<RefCell<Value>>> {
        &self.upvals
    }
    #[inline(always)]
    pub(super) fn getcells_mut(&mut self) -> &mut Vec<Rc<RefCell<Value>>> {
        &mut self.upvals
    }
    #[inline(always)]
    pub(super) fn getvar(&self, var: &Variable) -> Result<Value> {
        let value = match var.type_() {
            VariableType::Local => self.locals[var.slot()].clone(),
            VariableType::Upval => self.upvals[var.slot()].borrow().clone(),
        };
        if let Value::Invalid = value {
            Err(rterr!("Variable {:?} used before being set", var.name()))
        } else {
            Ok(value)
        }
    }
    #[inline(always)]
    pub(super) fn setargs(&mut self, params: &Vec<Variable>, args: Vec<Value>) {
        assert_eq!(params.len(), args.len());
        for (var, arg) in params.iter().zip(args) {
            self.setvar(var, arg);
        }
    }
    #[inline(always)]
    pub(super) fn fetch<'a>(&mut self, code: &'a Code) -> (usize, Option<&'a Opcode>) {
        let pc = self.pc;
        let opc = code.ops().get(pc);
        self.pc += 1;
        (pc, opc)
    }
    #[inline(always)]
    pub(super) fn jump(&mut self, pc: usize) {
        self.pc = pc;
    }
}
