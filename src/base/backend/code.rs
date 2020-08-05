use super::*;

pub struct Code {
    name: RcStr,
    ops: Vec<Opcode>,

    /// Describes how arguments map to the variables
    /// in a Frame
    params: Vec<Variable>,

    varspec: VarSpec,

    marks: Vec<Mark>,
}

impl fmt::Debug for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Code({:?})", self.name)
    }
}

impl Code {
    pub(crate) fn new(
        name: RcStr,
        ops: Vec<Opcode>,
        params: Vec<Variable>,
        varspec: VarSpec,
        marks: Vec<Mark>,
    ) -> Self {
        Self {
            name,
            ops,
            params,
            varspec,
            marks,
        }
    }
    pub fn name(&self) -> &RcStr {
        &self.name
    }
    pub(crate) fn ops(&self) -> &Vec<Opcode> {
        &self.ops
    }
    pub fn params(&self) -> &Vec<Variable> {
        &self.params
    }
    pub fn varspec(&self) -> &VarSpec {
        &self.varspec
    }
    pub fn marks(&self) -> &Vec<Mark> {
        &self.marks
    }
    pub(crate) fn new_frame(&self, bindings: Vec<Rc<RefCell<Value>>>) -> Frame {
        Frame::new(
            self.varspec.local().len(),
            bindings,
            self.varspec.owned().len(),
        )
    }

    /// For function calls
    /// runs this code object with the given bindings and arguments
    pub fn apply_for_function(
        &self,
        globals: &mut Globals,
        bindings: Vec<Rc<RefCell<Value>>>,
        args: Vec<Value>,
    ) -> Result<Value> {
        let mut frame = self.new_frame(bindings);
        frame.setargs(self.params(), args);
        self.run_frame(globals, &mut frame)
    }

    /// For modules
    /// runs this code object with the given map of builtins
    pub fn apply_for_module(
        &self,
        globals: &mut Globals,
        map: &HashMap<RcStr, Value>,
    ) -> Result<Rc<Module>> {
        assert_eq!(self.params.len(), 0);
        let mut input_bindings = Vec::new();
        for (name, mark) in self.varspec.free() {
            if let Some(value) = map.get(name) {
                input_bindings.push(Rc::new(RefCell::new(value.clone())));
            } else {
                return Err(crate::Error::rt(
                    format!("Name {:?} not found", name).into(),
                    vec![mark.clone()],
                ));
            }
        }
        let mut frame = self.new_frame(input_bindings);

        let freelen = self.varspec.free().len();
        let ownedlen = self.varspec.owned().len();
        let owned_bindings = frame.getcells()[freelen..][..ownedlen].to_vec();
        let module = Rc::new(Module::new_with_cells(
            self.name.clone(),
            self.varspec
                .owned()
                .iter()
                .map(|(name, _)| name.clone())
                .zip(owned_bindings)
                .collect(),
        ));
        globals.register_module(module.clone())?;

        self.run_frame(globals, &mut frame)?;
        Ok(module)
    }

    /// For repl
    /// runs this code object while using the given map essentially as a dynamic module scope
    pub fn apply_for_repl(&self, globals: &mut Globals) -> Result<Value> {
        assert_eq!(self.params.len(), 0);
        let repl_scope = globals.repl_scope_mut();
        let mut input_bindings = Vec::new();
        for (name, mark) in self.varspec.free() {
            if let Some(cell) = repl_scope.get(name) {
                input_bindings.push(cell.clone());
            } else {
                return Err(crate::Error::rt(
                    format!("Name {:?} not found", name).into(),
                    vec![mark.clone()],
                ));
            }
        }
        let mut frame = self.new_frame(input_bindings);

        // Sync owned cells between the frame and the map
        let freelen = self.varspec.free().len();
        let ownedlen = self.varspec.owned().len();
        let owned_entries = self.varspec.owned().clone();
        let owned_cells = &mut frame.getcells_mut()[freelen..][..ownedlen];
        for ((name, _mark), new_cell) in owned_entries.into_iter().zip(owned_cells) {
            if let Some(old_cell) = repl_scope.get(&name) {
                *new_cell = old_cell.clone();
            } else {
                repl_scope.insert(name, new_cell.clone());
            }
        }

        self.run_frame(globals, &mut frame)
    }

    fn run_frame(&self, globals: &mut Globals, frame: &mut Frame) -> Result<Value> {
        loop {
            match step(globals, self, frame) {
                StepResult::Ok => {}
                StepResult::Yield(_) => return Err(rterr!("Yield from unyieldable context")),
                StepResult::Return(value) => return Ok(value),
                StepResult::Err(error) => return Err(error),
            }
        }
    }

    pub(crate) fn resume_frame(&self, globals: &mut Globals, frame: &mut Frame, arg: Value) -> ResumeResult {
        frame.push(arg);
        loop {
            match step(globals, self, frame) {
                StepResult::Ok => {}
                StepResult::Yield(value) => return ResumeResult::Yield(value),
                StepResult::Return(value) => return ResumeResult::Return(value),
                StepResult::Err(error) => return ResumeResult::Err(error),
            }
        }
    }
}
