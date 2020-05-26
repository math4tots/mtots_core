/// Opcodes inspired by https://docs.python.org/3/library/dis.html
use super::VariableLocation;
use crate::Binop;
use crate::Class;
use crate::ClassKind;
use crate::Code;
use crate::ErrorIndicator;
use crate::Eval;
use crate::EvalError;
use crate::ExceptionKind;
use crate::Frame;
use crate::Function;
use crate::Globals;
use crate::Operation;
use crate::RcStr;
use crate::Table;
use crate::VMap;
use crate::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

macro_rules! count_ident {
    (()) => {
        0
    };
    (($arg0:ident : $type0:ident)) => {
        1
    };
    (($arg0:ident : $type0:ident, $arg1:ident : $type1:ident)) => {
        2
    };
    (($arg0:ident : $type0:ident, $arg1:ident : $type1:ident, $arg2:ident : $type2:ident)) => {
        3
    };
}

macro_rules! get_arg_types {
    (()) => {
        &[]
    };
    (($arg0:ident : $type0:ident)) => {
        &[OpcodeArgumentType::$type0]
    };
    (($arg0:ident : $type0:ident, $arg1:ident : $type1:ident)) => {
        &[OpcodeArgumentType::$type0, OpcodeArgumentType::$type1]
    };
    (($arg0:ident : $type0:ident, $arg1:ident : $type1:ident, $arg2:ident : $type2:ident)) => {
        &[
            OpcodeArgumentType::$type0,
            OpcodeArgumentType::$type1,
            OpcodeArgumentType::$type2,
        ]
    };
}

macro_rules! define_args {
    ($code:ident, $frame:ident, ()) => {};
    ($code:ident, $frame:ident,
            ($arg0:ident : $type0:ident)) => {
        let $arg0 = $code.code[$frame.i + 1];
    };
    ($code:ident, $frame:ident,
            ($arg0:ident : $type0:ident, $arg1:ident : $type1:ident)) => {
        let $arg0 = $code.code[$frame.i + 1];
        let $arg1 = $code.code[$frame.i + 2];
    };
    ($code:ident, $frame:ident,
            ($arg0:ident : $type0:ident,
                $arg1:ident : $type1:ident,
                $arg2:ident : $type2:ident)) => {
        let $arg0 = $code.code[$frame.i + 1];
        let $arg1 = $code.code[$frame.i + 2];
        let $arg2 = $code.code[$frame.i + 3];
    };
}

macro_rules! define_args_for_branches_func {
    ($args:ident, ()) => {};
    ($args:ident,
            ($arg0:ident : $type0:ident)) => {
        let $arg0 = $args[0];
    };
    ($args:ident,
            ($arg0:ident : $type0:ident,
                $arg1:ident : $type1:ident)) => {
        let $arg0 = $args[0];
        let $arg1 = $args[1];
    };
    ($args:ident,
            ($arg0:ident : $type0:ident,
                $arg1:ident : $type1:ident,
                $arg2:ident : $type2:ident)) => {
        let $arg0 = $args[0];
        let $arg1 = $args[1];
        let $arg2 = $args[2];
    };
}

macro_rules! branches_func {
    (
        $ip:ident, $args:tt, [ + $popcnt:tt $pushcnt:tt ]
    ) => { {
        #[allow(unused)]
        fn branches_func(args: &[usize], $ip: usize) -> Vec<(usize, usize, usize)> {
            define_args_for_branches_func!(args, $args);
            vec![($ip + 1, $popcnt, $pushcnt)]
        }
        branches_func
    } };
    (
        $ip:ident, $args:tt, [ ; $($ipexpr:tt $popcnt:tt $pushcnt:tt),+ ]
    ) => { {
        #[allow(unused)]
        fn branches_func(args: &[usize], $ip: usize) -> Vec<(usize, usize, usize)> {
            define_args_for_branches_func!(args, $args);
            vec![$( ($ipexpr, $popcnt, $pushcnt) ),*]
        }
        branches_func
    } };
}

pub(super) enum StepException {
    Error,
    Yield,
    Return,
}

impl From<ErrorIndicator> for StepException {
    fn from(_: ErrorIndicator) -> StepException {
        StepException::Error
    }
}

macro_rules! define_opcodes {
    (
        globals = $globals:ident,
        frame = $frame:ident,
        code = $code:ident,
        ip = $ip:ident,
        ARGC = $argc:ident,
        ;
        $( $name:ident $args:tt $jmp_and_stack_info:tt $body:block )*
    ) => {
        #[allow(non_camel_case_types)]
        enum OpcodeNumbering {
            $( $name, )*
        }
        $( pub const $name: usize = OpcodeNumbering::$name as usize; )*

        pub const OPCODE_INFO_MAP: &[OpcodeInfo] = &[
            $(
                OpcodeInfo {
                    name: stringify!($name),
                    argtypes: get_arg_types!($args),
                    branches: branches_func!($ip, $args, $jmp_and_stack_info),
                },
            )*
        ];

        /// `step` contains logic for executing a single bytecode instruction.
        /// It needs to get factored out for e.g. the normal function case vs
        /// generator function cases.
        /// It's pretty obvious that this is going to be called heavily in
        /// tight loops, so we mark it inline(always).
        #[inline(always)]
        #[allow(dead_code)]
        pub(super) fn step(
            $globals: &mut Globals,
            $frame: &mut Frame,
            $code: &Code,
        ) -> Result<(), StepException> {
            let opcode = $code.code[$frame.i];
            match opcode {
                $(
                    $name => {
                        const $argc: usize = count_ident!($args);
                        define_args!($code, $frame, $args);
                        $frame.i += 1 + count_ident!($args);
                        $body
                    }
                )*
                _ => panic!("Invalid opcode {}", opcode),
            }
            Ok(())
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OpcodeArgumentType {
    Int,   // just used a number
    Const, // index into constants list
    Label, // jump location
    Local, // index into Frame::locals
    Cell,  // index into Frame::cellvars
    Code,  // index into the childrens list
    Name,  // various name args saved in the 'names' list

    // for situations where line numbers are required for the normal operation of an opcode
    // for example, with IMPORT or CALL_FUNCTION, regardless of whether the operation succeeds,
    // the opcode must add an entry to the trace, since the trace may be used in the call
    // itself. Code objects generally can get line numbers for any given offset in their
    // opcodes list, but that process could potentially be very slow.
    LineNumber,
}

impl OpcodeArgumentType {
    pub(super) fn as_var_loc(&self, arg: usize) -> Option<VariableLocation> {
        match self {
            OpcodeArgumentType::Local => Some(VariableLocation::Local(arg)),
            OpcodeArgumentType::Cell => Some(VariableLocation::Cell(arg)),
            _ => None,
        }
    }
}

#[allow(dead_code)]
pub struct OpcodeInfo {
    name: &'static str,

    // the number of arguments this opcode expects
    argtypes: &'static [OpcodeArgumentType],

    // given:
    //   the opcode arguments (&[usize]), and
    //   the current instruction pointer (usize),
    // returns a vector of possible (new-ip, pop-count, push-count) triples
    // when this opcode executes
    branches: fn(&[usize], usize) -> Vec<(usize, usize, usize)>,
}

impl OpcodeInfo {
    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn argtypes(&self) -> &'static [OpcodeArgumentType] {
        self.argtypes
    }

    #[allow(dead_code)]
    pub fn branches_func(&self) -> fn(&[usize], usize) -> Vec<(usize, usize, usize)> {
        self.branches
    }
}

macro_rules! defbinop {
    (
        $lineno:ident, $globals:ident, $code:ident, $frame:ident, $op:ident, $( $pat:pat => $val:expr , )*
    ) => {
        let rhs = $frame.stack.pop().unwrap();
        let lhs = $frame.stack.pop().unwrap();
        $frame.stack.push(match (&lhs, &rhs) {
            $( $pat => $val , )*
            _ => {
                $globals.trace_push($code.module_name.clone(), $lineno);
                return $globals.set_exc_legacy(EvalError::OperationNotSupportedForKinds(
                    Operation::Binop(Binop::$op),
                    vec![lhs.kind(), rhs.kind()]))?;
            }
        });
    };
}

define_opcodes! { globals = globals, frame = frame, code = code, ip = ip, ARGC = ARGC, ;
    // <OpcodeName> (<args>)
    // ([ ; (potential-new-ip-values <pop-count> <push-count>),* ])?
    // { <body> }
    // <pop-count> does not always actually mean that the values are popped
    // -- it's meant to indicate the number of values that are read from the top
    // of the stack. However, when computing the stack size, it's treated
    // as if that many elements had been popped and <push-count> elements
    // are pushed after
    NOP() [+ 0 0] {}
    POP_TOP() [+ 1 0] {
        frame.stack.pop().unwrap();
    }
    ROT_TWO() [+ 2 2] {
        let len = frame.stack.len();
        frame.stack.swap(len - 1, len - 2);
    }
    PULL_TOS3() [+ 4 4] {
        // pulls TOS3 (4th element from the top of the stack) out,
        // and puts it on top, so it becomes the new TOS, with all
        // elements that used to be on top of it one position down
        let len = frame.stack.len();
        let tos3 = frame.stack.remove(len - 4);
        frame.stack.push(tos3);
    }
    DUP_TOP() [+ 1 2] {
        let top = frame.stack.last().unwrap().clone();
        frame.stack.push(top);
    }
    LOAD_CONST(i: Const) [+ 0 1] {
        let constant = code.constants[i].clone();
        frame.stack.push(constant);
    }

    LOAD_LOCAL(i: Local) [+ 0 1] {
        frame.stack.push(frame.locals[i].clone());
    }
    STORE_LOCAL(i: Local) [+ 1 0] {
        frame.locals[i] = frame.stack.pop().unwrap();
    }
    LOAD_DEREF(i: Cell) [+ 0 1] {
        frame.stack.push(frame.cellvars[i].borrow().clone());
    }
    STORE_DEREF(i: Cell) [+ 1 0] {
        *frame.cellvars[i].borrow_mut() = frame.stack.pop().unwrap();
    }
    LOAD_CELL(i: Cell) [+ 0 1] {
        frame.stack.push(Value::Cell(frame.cellvars[i].clone()));
    }

    UNPACK_SEQUENCE(n: Int) [+ 1 n] {
        // Unpacks TOS into count individual values,
        // which are put onto the stack right-to-left
        match frame.stack.pop().unwrap() {
            Value::List(list) => {
                if list.len() != n {
                    let err = EvalError::UnpackSize {
                        expected: list.len(),
                        but_got: n,
                    };
                    let lineno = code.find_lineno_for_opcode_at(frame.i - 1 - ARGC);
                    globals.trace_push(code.module_name.clone(), lineno);
                    return globals.set_exc_legacy(err.into())?;
                }
                frame.stack.extend(list.iter().rev().map(|v| v.clone()));
            }
            iterable => {
                let items = Eval::iterable_to_vec(globals, &iterable)?;
                if items.len() != n {
                    let err = EvalError::UnpackSize {
                        expected: items.len(),
                        but_got: n,
                    };
                    let lineno = code.find_lineno_for_opcode_at(frame.i - 1 - ARGC);
                    globals.trace_push(code.module_name.clone(), lineno);
                    return globals.set_exc_legacy(err.into())?;
                }
                frame.stack.extend(items.into_iter().rev());
            }
        }
    }

    MAKE_MUTABLE_STRING(namei: Name) [+ 0 1] {
        let string = code.names[namei];
        frame.stack.push(Value::MutableString(RefCell::new(string.str().to_owned()).into()));
    }
    MAKE_LIST(argc: Int) [+ argc 1] {
        let len = frame.stack.len();
        let args = frame.stack.split_off(len - argc);
        frame.stack.push(Value::List(args.into()));
    }
    MAKE_TABLE(argc: Int) [+ (2 * argc) 1] {
        let len = frame.stack.len();
        let args = frame.stack.split_off(len - (2 * argc));
        let mut map = HashMap::new();
        for pair in args.chunks(2) {
            let key = Eval::expect_symbol(globals, &pair[0])?;
            map.insert(key, pair[1].clone());
        }
        frame.stack.push(Value::Table(Table::new(map).into()));
    }
    MAKE_MAP(argc: Int) [+ (2 * argc) 1] {
        let len = frame.stack.len();
        let args = frame.stack.split_off(len - (2 * argc));
        let mut map = VMap::new();
        for pair in args.chunks(2) {
            map.s_insert(globals, pair[0].clone(), pair[1].clone())?;
        }
        frame.stack.push(map.into());
    }
    MAKE_MUTABLE_LIST(argc: Int) [+ argc 1] {
        let len = frame.stack.len();
        let args = frame.stack.split_off(len - argc);
        frame.stack.push(Value::MutableList(RefCell::new(args).into()));
    }
    MAKE_MUTABLE_MAP(argc: Int) [+ (2 * argc) 1] {
        let len = frame.stack.len();
        let args = frame.stack.split_off(len - (2 * argc));
        let mut map = VMap::new();
        for pair in args.chunks(2) {
            map.s_insert(globals, pair[0].clone(), pair[1].clone())?;
        }
        frame.stack.push(Value::MutableMap(RefCell::new(map).into()));
    }

    LOAD_METHOD(namei: Name) [+ 1 2] {
        // Pops TOS, pushes the unbound method for it with the given name,
        // then pushes the old TOS back onto the stack
        // i.e. [self] => [unbound-method, self]
        // This opcode is meant to be used with the 'CALL_FUNCTION' opcode
        let owner = frame.stack.pop().unwrap();
        let name = code.names[namei];
        let method = match Eval::get_method(globals, &owner, name) {
            Ok(method) => method,
            Err(error) => {
                let lineno = code.find_lineno_for_opcode_at(frame.i - 1 - ARGC);
                globals.trace_push(code.module_name.clone(), lineno);
                return Err(error)?;
            }
        };
        frame.stack.push(method.clone());
        frame.stack.push(owner);
    }

    CALL_FUNCTION(lineno: LineNumber, argc: Int) [+ (argc + 1) 1] {
        let len = frame.stack.len();
        let args = frame.stack.split_off(len - argc);
        let f = frame.stack.pop().unwrap();

        globals.trace_push(code.module_name.clone(), lineno);
        let value = Eval::call(globals, &f, args)?;
        globals.trace_pop();

        frame.stack.push(value);
    }

    CALL_FUNCTION_GENERIC(lineno: LineNumber) [+ 3 1] {
        // Instead of reading n elements from the stack, this version will
        // always pop exactly 3 values off the stack, where
        //   - TOS => keyword argument Table,
        //   - TOS1 => argument List,
        //   - TOS2 => the function
        // The Table and List arguments must also be unique references
        // This is to ensure that the function call does not require
        // a clone of these values, but can simply move out of them
        let kwargs = frame.stack.pop().unwrap();
        let args = frame.stack.pop().unwrap();
        let f = frame.stack.pop().unwrap();

        let args = Eval::move_list(globals, args)?;
        let kwargs = Eval::move_table(globals, kwargs)?.map_move();
        globals.trace_push(code.module_name.clone(), lineno);
        let value = Eval::call_with_kwargs(globals, &f, args, Some(kwargs))?;
        globals.trace_pop();

        frame.stack.push(value);
    }

    EXTEND_LIST(lineno: LineNumber) [+ 2 1] {
        // Primary purpose of this opcode is to use with CALL_FUNCTION_GENERIC
        // Expects TOS and TOS1 to be lists, pops them and pushes a new
        // list consisting of TOS1 extended with TOS
        //   - TOS must be an iterable
        //   - TOS1 must be a List
        //   - TOS1 must be a unique reference
        let tos = frame.stack.pop().unwrap();
        let tos1 = frame.stack.pop().unwrap();

        globals.trace_push(code.module_name.clone(), lineno);
        let mut args = Eval::move_list(globals, tos1)?;
        Eval::extend_from_iterable(globals, &mut args, &tos)?;
        globals.trace_pop();

        frame.stack.push(args.into());
    }

    EXTEND_TABLE(lineno: LineNumber) [+ 2 1] {
        // Table version of EXTEND_LIST
        // Primary purpose of this opcode is to use with CALL_FUNCTION_GENERIC
        //   - TOS must be a Table
        //   - TOS1 must be a Table
        //   - TOS1 must be a unique reference
        let tos = frame.stack.pop().unwrap();
        let tos1 = frame.stack.pop().unwrap();
        globals.trace_push(code.module_name.clone(), lineno);
        let mut kwargs = Eval::move_table(globals, tos1)?.map_move();
        kwargs.extend(
            Eval::expect_table(globals, &tos)?
                .map()
                .iter()
                .map(|(k, v)| (*k, v.clone())),
        );
        globals.trace_pop();
        frame.stack.push(Value::Table(Table::new(kwargs).into()))
    }

    MAKE_FUNCTION(i: Code) [+ 1 1] {
        let cells = if let Some(cells) = get_cells(frame.stack.pop().unwrap()) {
            cells
        } else {
            // Unless there's a bug in the Rust code, this should almost
            // never happen.
            // However, I decided not to panic here because if dynamically
            // generating bytecode is allowed, this can be triggered even
            // if there's no bug in the Rust code.
            return globals.set_exc_legacy(EvalError::MakeFunctionInvalidCellsValue)?;
        };
        let func_code = code.children[i].clone();
        frame.stack.push(Value::Function(Function::new(cells, func_code).into()));
    }

    MAKE_CLASS(full_name: Name, is_trait: Int) [+ 4 1] {
        // Expects:
        //   TOS : static member Table
        //   TOS1: instance method Table
        //   TOS2: List of field names (as symbols)
        //   TOS3: List of traits
        let kind = if is_trait != 0 { ClassKind::Trait } else { ClassKind::UserDefinedClass };
        let full_name = globals.symbol_rcstr(code.names[full_name]).clone();
        let len = frame.stack.len();
        let cls = new_class(globals, kind, full_name, &frame.stack[len - 4..])?;
        frame.stack.truncate(len - 4);
        frame.stack.push(cls.into());
    }

    MAKE_EXCEPTION_KIND(namei: Name) [+ 3 1] {
        // Expects:
        //   TOS : exception message template (as String)
        //   TOS1: field names (as List of Symbols or nil if none)
        //   TOS2: base exception kind (or nil for Exception)
        let full_name = globals.symbol_rcstr(code.names[namei]).clone();
        let len = frame.stack.len();
        let exckind = new_exc_kind(globals, full_name, &frame.stack[len - 3..])?;
        frame.stack.truncate(len - 3);
        frame.stack.push(Value::ExceptionKind(exckind));
    }

    JUMP(i: Label) [; i 0 0] {
        frame.i = i;
    }
    POP_JUMP_IF_TRUE(i: Label) [; (ip + 1) 1 0, i 1 0] {
        let value = frame.stack.pop().unwrap();
        if Eval::truthy(globals, &value)? {
            frame.i = i;
        }
    }
    POP_JUMP_IF_FALSE(i: Label) [; (ip + 1) 1 0, i 1 0] {
        let value = frame.stack.pop().unwrap();
        if !Eval::truthy(globals, &value)? {
            frame.i = i;
        }
    }
    JUMP_IF_TRUE_OR_POP(i: Label) [; (ip + 1) 1 1, i 1 0] {
        if Eval::truthy(globals, frame.stack.last().unwrap())? {
            frame.i = i;
        } else {
            frame.stack.pop().unwrap();
        }
    }
    JUMP_IF_FALSE_OR_POP(i: Label) [; (ip + 1) 1 1, i 1 0] {
        if !Eval::truthy(globals, frame.stack.last().unwrap())? {
            frame.i = i;
        } else {
            frame.stack.pop().unwrap();
        }
    }

    GET_ITER() [+ 1 1] {
        // TOS = iter(TOS)
        let iter = Eval::iter(globals, &frame.stack.pop().unwrap())?;
        frame.stack.push(iter);
    }
    FOR_ITER(i: Label) [; (ip + 1) 1 2, i 1 0] {
        // TOS must be an iterator,
        //   - if TOS is non-empty, get next element and push it onto the stack
        //     the iterator is not popped from the stack
        //   - if TOS is empty, pop the iterator and jump to i
        let iterator = frame.stack.last().unwrap();

        if let Some(value) = Eval::next(globals, iterator)? {
            frame.stack.push(value);
        } else {
            frame.stack.pop();
            frame.i = i;
        }
    }

    BINARY_POWER(lineno: LineNumber) [+ 2 1] {
        defbinop! { lineno, globals, code, frame, Pow,
            (Value::Int(a), Value::Int(b)) => {
                if *b >= 0 && *b <= (std::u32::MAX as i64) {
                    Value::Int(a.pow(*b as u32))
                } else {
                    Value::Float((*a as f64).powf(*b as f64))
                }
            },
        }
    }
    BINARY_ADD(lineno: LineNumber) [+ 2 1] {
        let rhs = frame.stack.pop().unwrap();
        let lhs = frame.stack.pop().unwrap();
        frame.stack.push(Eval::add0(globals, lhs, rhs, Some((code, lineno)))?);
    }
    BINARY_SUB(lineno: LineNumber) [+ 2 1] {
        let rhs = frame.stack.pop().unwrap();
        let lhs = frame.stack.pop().unwrap();
        frame.stack.push(Eval::sub0(globals, lhs, rhs, Some((code, lineno)))?);
    }
    BINARY_MUL(lineno: LineNumber) [+ 2 1] {
        let rhs = frame.stack.pop().unwrap();
        let lhs = frame.stack.pop().unwrap();
        frame.stack.push(Eval::mul0(globals, lhs, rhs, Some((code, lineno)))?);
    }
    BINARY_DIV(lineno: LineNumber) [+ 2 1] {
        let rhs = frame.stack.pop().unwrap();
        let lhs = frame.stack.pop().unwrap();
        frame.stack.push(Eval::div0(globals, lhs, rhs, Some((code, lineno)))?);
    }
    BINARY_TRUNCDIV(lineno: LineNumber) [+ 2 1] {
        let rhs = frame.stack.pop().unwrap();
        let lhs = frame.stack.pop().unwrap();
        frame.stack.push(Eval::truncdiv0(globals, lhs, rhs, Some((code, lineno)))?);
    }
    BINARY_REM(lineno: LineNumber) [+ 2 1] {
        let rhs = frame.stack.pop().unwrap();
        let lhs = frame.stack.pop().unwrap();
        frame.stack.push(Eval::rem0(globals, lhs, rhs, Some((code, lineno)))?);
    }
    BINARY_EQ(lineno: LineNumber) [+ 2 1] {
        let rhs = frame.stack.pop().unwrap();
        let lhs = frame.stack.pop().unwrap();
        frame.stack.push(Value::Bool(Eval::eq0(globals, &lhs, &rhs, Some((code, lineno)))?));
    }
    BINARY_LT(lineno: LineNumber) [+ 2 1] {
        let rhs = frame.stack.pop().unwrap();
        let lhs = frame.stack.pop().unwrap();
        frame.stack.push(Value::Bool(Eval::lt0(globals, &lhs, &rhs, Some((code, lineno)))?));
    }
    BINARY_IS() [+ 2 1] {
        let a = frame.stack.pop().unwrap();
        let b = frame.stack.pop().unwrap();
        frame.stack.push(a.is(&b).into());
    }

    UNARY_NOT(lineno: LineNumber) [+ 1 1] {
        let value = frame.stack.pop().unwrap();
        let truthy = Eval::truthy0(globals, &value, Some((code, lineno)))?;
        frame.stack.push((!truthy).into());
    }
    UNARY_NEG(lineno: LineNumber) [+ 1 1] {
        let value = frame.stack.pop().unwrap();
        frame.stack.push(Eval::neg0(globals, &value, Some((code, lineno)))?);
    }
    UNARY_POS(lineno: LineNumber) [+ 1 1] {
        let value = frame.stack.pop().unwrap();
        frame.stack.push(Eval::pos0(globals, &value, Some((code, lineno)))?);
    }

    LOAD_ATTRIBUTE(i: Name) [+ 1 1] {
        let owner = frame.stack.pop().unwrap();
        let name = code.names[i];
        if let Some(value) = Eval::getattr(globals, &owner, name) {
            frame.stack.push(value);
        } else {
            let error = EvalError::NoSuchAttribute(name);
            let lineno = code.find_lineno_for_opcode_at(frame.i - 1 - ARGC);
            globals.trace_push(code.module_name.clone(), lineno);
            return globals.set_exc_legacy(error.into())?;
        }
    }
    STORE_ATTRIBUTE(i: Name) [+ 2 0] {
        let owner = frame.stack.pop().unwrap();
        let value = frame.stack.pop().unwrap();
        let name = code.names[i];
        if Eval::setattr(globals, &owner, name, value).is_err() {
            let error = EvalError::CouldNotAssignAttribute(name);
            let lineno = code.find_lineno_for_opcode_at(frame.i - 1 - ARGC);
            globals.trace_push(code.module_name.clone(), lineno);
            return globals.set_exc_legacy(error.into())?;
        }
    }
    LOAD_STATIC_ATTRIBUTE(i: Name) [+ 1 1] {
        let owner = frame.stack.pop().unwrap();
        let name = code.names[i];
        if let Some(value) = Eval::get_static_attr(globals, &owner, name) {
            frame.stack.push(value);
        } else {
            let lineno = code.find_lineno_for_opcode_at(frame.i - 1 - ARGC);
            globals.trace_push(code.module_name.clone(), lineno);
            return globals.set_static_attr_error(name, owner.clone())?;
        }
    }

    YIELD() [+ 1 1] {
        return Err(StepException::Yield);
    }
    RETURN() [+ 1 0] {
        return Err(StepException::Return);
    }

    BREAKPOINT() [+ 0 1] {
        enter_breakpoint(globals, frame, code)?;
    }

    IMPORT(lineno: LineNumber, namei: Name) [+ 2 1] {
        let name = code.names[namei];

        globals.trace_push(code.module_name.clone(), lineno);
        let module = globals.load_by_symbol(name)?;
        globals.trace_pop();

        frame.stack.push(Value::Module(module));
    }
}

fn step_noinline(
    globals: &mut Globals,
    frame: &mut Frame,
    code: &Code,
) -> Result<(), StepException> {
    step(globals, frame, code)
}

fn get_cells(value: Value) -> Option<Vec<Rc<RefCell<Value>>>> {
    let list = value.list()?;
    let mut cells = Vec::new();
    for x in list.iter() {
        cells.push(x.cell()?.clone());
    }
    Some(cells)
}

fn new_class(
    globals: &mut Globals,
    kind: ClassKind,
    full_name: RcStr,
    args: &[Value],
) -> Result<Rc<Class>, StepException> {
    let bases = {
        let mut bases = Vec::new();
        for base in Eval::expect_list(globals, &args[0])?.iter() {
            let base = Eval::expect_class(globals, base)?;
            bases.push(base.clone());
        }
        bases
    };
    let fields = {
        let mut fields = Vec::new();
        for field in Eval::expect_list(globals, &args[1])?.iter() {
            let field = Eval::expect_symbol(globals, field)?;
            fields.push(field);
        }
        match kind {
            ClassKind::NativeClass | ClassKind::Trait => {
                if fields.is_empty() {
                    None
                } else {
                    return globals
                        .set_exc_other("field lists are not allowed on traits".into())?;
                }
            }
            ClassKind::UserDefinedClass => Some(fields),
        }
    };
    let map = Eval::expect_table(globals, &args[2])?.map().clone();
    let static_map = Eval::expect_table(globals, &args[3])?.map().clone();
    let cls = Class::new(globals, kind, full_name, bases, fields, map, static_map)?;
    Ok(cls)
}

fn new_exc_kind(
    globals: &mut Globals,
    full_name: RcStr,
    args: &[Value],
) -> Result<Rc<ExceptionKind>, StepException> {
    let base = if let Value::Nil = &args[0] {
        globals.builtin_exceptions().Exception.clone()
    } else {
        Eval::expect_exception_kind(globals, &args[0])?.clone()
    };
    let fields = if let Value::Nil = &args[1] {
        None
    } else {
        let mut fields = Vec::new();
        for field in Eval::expect_list(globals, &args[1])?.iter() {
            let field = Eval::expect_symbol(globals, field)?;
            let field = globals.symbol_rcstr(field);
            fields.push(field);
        }
        Some(fields)
    };
    let message = Eval::expect_string(globals, &args[2])?.clone();
    Ok(globals.new_exc_kind(base, full_name, message, fields))
}

fn enter_breakpoint(
    globals: &mut Globals,
    frame: &mut Frame,
    code: &Code,
) -> Result<(), StepException> {
    eprintln!("#### Entering breakpoint ####");
    let offset = frame.i;
    eprintln!(
        "#### lineno = {}, bytecode-offset = {} ####",
        code.find_lineno_for_opcode_at(offset),
        offset
    );
    loop {
        eprint!(">> ");
        let mut line = String::new();
        if let Err(error) = std::io::stdin().read_line(&mut line) {
            return globals.set_exc_legacy(EvalError::IOError(error))?;
        }
        let cmd = line.trim();
        match cmd {
            // continue the program
            "c" => break,

            // step -- run one instruction
            // if the function throws a StepException, we might fall off
            // the breakpoint
            "s" => {
                step_noinline(globals, frame, code)?;
            }

            // print the operand stack
            "ps" => {
                eprintln!("{:?}", frame.stack);
            }

            // print the traceback
            "pt" => {
                let lineno = code.find_lineno_for_opcode_at(frame.i);
                globals.trace_push(code.module_name.clone(), lineno);
                eprint!("{}", globals.trace_str());
                globals.trace_pop();
            }

            // print the code object
            "pc" => {
                eprintln!("{}", code.debugstr0());
            }

            // print the current instruction index
            "pi" => {
                eprintln!("{}", frame.i);
            }

            _ => {
                eprintln!("Unrecognized debug command {:?}", cmd);
            }
        }
    }
    Ok(())
}
