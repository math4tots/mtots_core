use crate::Eval;
use crate::EvalResult;
use crate::Globals;
use crate::HMap;
use crate::NativeFunction;
use crate::RcStr;
use crate::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::rc::Rc;

pub const NAME: &str = "a._os";

const OS: &str = std::env::consts::OS;
const FAMILY: &str = std::env::consts::FAMILY;
const ARCH: &str = std::env::consts::ARCH;

pub(super) fn load(globals: &mut Globals) -> EvalResult<HMap<RcStr, Rc<RefCell<Value>>>> {
    let sr = globals.symbol_registry();
    let mut map = HashMap::<RcStr, Value>::new();
    map.insert("name".into(), OS.into());
    map.insert("family".into(), FAMILY.into());
    map.insert("arch".into(), ARCH.into());

    map.insert("sep".into(), std::path::MAIN_SEPARATOR.to_string().into());

    map.extend(
        vec![
            NativeFunction::simple0(sr, "getcwd", &[], |globals, _, _| {
                let cwd = Eval::try_(globals, std::env::current_dir())?;
                Ok(cwd.into())
            }),
            NativeFunction::simple0(sr, "env", &["name"], |globals, args, _kwargs| {
                let name = Eval::expect_osstr(globals, &args[0])?;
                match env::var(name) {
                    Ok(value) => Ok(value.into()),
                    Err(env::VarError::NotPresent) => Ok(Value::Nil),
                    Err(env::VarError::NotUnicode(s)) => {
                        globals.set_exc_str(&format!("Not unicode: {:?}", s))
                    }
                }
            }),
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
