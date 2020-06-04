use crate::Eval;
use crate::EvalResult;
use crate::Globals;
use crate::HMap;
use crate::NativeFunction;
use crate::RcStr;
use crate::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::SystemTime;

pub const NAME: &str = "time";

pub(super) fn load(globals: &mut Globals) -> EvalResult<HMap<RcStr, Rc<RefCell<Value>>>> {
    let sr = globals.symbol_registry();
    let mut map = HashMap::<RcStr, Value>::new();

    map.extend(
        vec![
            NativeFunction::simple0(sr, "now", &[], |_globals, _args, _kwargs| {
                // Returns the current time as a float -- secs since UNIX EPOCH
                let ts = diff_system_times(SystemTime::UNIX_EPOCH, SystemTime::now());
                Ok(ts.into())
            }),
            NativeFunction::simple0(sr, "sleep", &["sec"], |globals, args, _kwargs| {
                // Sleeps for given number of secs (expects int or float)
                let sec = Eval::expect_floatlike(globals, &args[0])?;
                std::thread::sleep(std::time::Duration::from_secs_f64(sec));
                Ok(Value::Nil)
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

fn diff_system_times(start: SystemTime, end: SystemTime) -> f64 {
    match end.duration_since(start) {
        Ok(duration) => duration.as_secs_f64(),
        Err(error) => -error.duration().as_secs_f64(),
    }
}
