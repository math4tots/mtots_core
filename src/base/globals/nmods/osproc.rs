use crate::Eval;
use crate::EvalResult;
use crate::Globals;
use crate::HMap;
use crate::NativeFunction;
use crate::Opaque;
use crate::RcStr;
use crate::Symbol;
use crate::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::process::Child;
use std::rc::Rc;

pub const NAME: &str = "_os.proc";

pub(super) fn load(globals: &mut Globals) -> EvalResult<HMap<RcStr, Rc<RefCell<Value>>>> {
    let sr = globals.symbol_registry();
    let mut map = HashMap::<RcStr, Value>::new();

    map.extend(
        vec![
            NativeFunction::snew(
                sr,
                "spawn",
                (
                    &["cmd", "args", "stdin", "stdout", "stderr"],
                    &[],
                    None,
                    None,
                ),
                |globals, args, _kwargs| {
                    let cmd = Eval::expect_string(globals, &args[0])?;
                    let mut cmd = std::process::Command::new(cmd.str());
                    if let Value::Nil = &args[1] {
                        // nil args mean no args
                    } else {
                        for arg in Eval::expect_list(globals, &args[1])? {
                            let argstr = Eval::expect_string(globals, arg)?;
                            cmd.arg(argstr.str());
                        }
                    }

                    let stdin = Eval::expect_symbol(globals, &args[2])?;
                    let stdout = Eval::expect_symbol(globals, &args[3])?;
                    let stderr = Eval::expect_symbol(globals, &args[4])?;

                    let stdin = translate_symbol_to_stdio(globals, stdin)?;
                    let stdout = translate_symbol_to_stdio(globals, stdout)?;
                    let stderr = translate_symbol_to_stdio(globals, stderr)?;

                    cmd.stdin(stdin);
                    cmd.stdout(stdout);
                    cmd.stderr(stderr);

                    let child: Child = match cmd.spawn() {
                        Ok(child) => child,
                        Err(error) => return globals.set_io_error(error),
                    };

                    Ok(Opaque::new(child).into())
                },
            ),
            NativeFunction::snew(
                sr,
                "wait",
                (&["child_proc"], &[], None, None),
                |globals, args, _kwargs| {
                    let child = Eval::move_opaque::<Child>(globals, &args[0])?;
                    let output = match child.wait_with_output() {
                        Ok(output) => output,
                        Err(error) => {
                            return globals.set_io_error(error);
                        }
                    };
                    let status = output
                        .status
                        .code()
                        .map(|c| Value::Int(c as i64))
                        .unwrap_or(Value::Nil);
                    let stdout = Value::Bytes(output.stdout.into());
                    let stderr = Value::Bytes(output.stderr.into());
                    Ok(vec![status, stdout, stderr].into())
                },
            ),
        ]
        .into_iter()
        .map(|f| (f.name().clone(), f.into())),
    );

    Ok({
        let mut ret = HMap::new();
        for (key, value) in map {
            ret.insert(key, Rc::new(RefCell::new(value)));
        }
        ret
    })
}

fn translate_symbol_to_stdio(
    globals: &mut Globals,
    symbol: Symbol,
) -> EvalResult<std::process::Stdio> {
    if symbol == Symbol::INHERIT {
        Ok(std::process::Stdio::inherit())
    } else if symbol == Symbol::PIPE {
        Ok(std::process::Stdio::piped())
    } else if symbol == Symbol::NULL {
        Ok(std::process::Stdio::null())
    } else {
        globals.set_exc_str(&format!(
            "Expected :inherit, :pipe or :null but got :{}",
            symbol.str(),
        ))
    }
}
