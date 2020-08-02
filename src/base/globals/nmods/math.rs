//! three bindings
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

pub const NAME: &str = "a._math";

pub(super) fn load(_globals: &mut Globals) -> EvalResult<HMap<RcStr, Rc<RefCell<Value>>>> {
    let mut map = HashMap::<RcStr, Value>::new();

    map.extend(
        vec![
            NativeFunction::simple0("sin", &["x"], |globals, args, _| {
                let x = Eval::expect_floatlike(globals, &args[0])?;
                Ok(x.sin().into())
            }),
            NativeFunction::simple0("cos", &["x"], |globals, args, _| {
                let x = Eval::expect_floatlike(globals, &args[0])?;
                Ok(x.cos().into())
            }),
            NativeFunction::simple0("tan", &["x"], |globals, args, _| {
                let x = Eval::expect_floatlike(globals, &args[0])?;
                Ok(x.tan().into())
            }),
            NativeFunction::simple0("asin", &["x"], |globals, args, _| {
                let x = Eval::expect_floatlike(globals, &args[0])?;
                Ok(x.asin().into())
            }),
            NativeFunction::simple0("acos", &["x"], |globals, args, _| {
                let x = Eval::expect_floatlike(globals, &args[0])?;
                Ok(x.acos().into())
            }),
            NativeFunction::simple0("atan", &["x"], |globals, args, _| {
                let x = Eval::expect_floatlike(globals, &args[0])?;
                Ok(x.atan().into())
            }),
            NativeFunction::simple0("atan2", &["y", "x"], |globals, args, _| {
                let y = Eval::expect_floatlike(globals, &args[0])?;
                let x = Eval::expect_floatlike(globals, &args[1])?;
                Ok(y.atan2(x).into())
            }),
            NativeFunction::simple0("sinh", &["x"], |globals, args, _| {
                let x = Eval::expect_floatlike(globals, &args[0])?;
                Ok(x.sinh().into())
            }),
            NativeFunction::simple0("cosh", &["x"], |globals, args, _| {
                let x = Eval::expect_floatlike(globals, &args[0])?;
                Ok(x.cosh().into())
            }),
            NativeFunction::simple0("tanh", &["x"], |globals, args, _| {
                let x = Eval::expect_floatlike(globals, &args[0])?;
                Ok(x.tanh().into())
            }),
            NativeFunction::simple0("ln", &["x"], |globals, args, _| {
                let x = Eval::expect_floatlike(globals, &args[0])?;
                Ok(x.ln().into())
            }),
            NativeFunction::simple0("log2", &["x"], |globals, args, _| {
                let x = Eval::expect_floatlike(globals, &args[0])?;
                Ok(x.log2().into())
            }),
            NativeFunction::simple0("log10", &["x"], |globals, args, _| {
                let x = Eval::expect_floatlike(globals, &args[0])?;
                Ok(x.log10().into())
            }),
            NativeFunction::simple0("log", &["x", "base"], |globals, args, _| {
                let x = Eval::expect_floatlike(globals, &args[0])?;
                let y = Eval::expect_floatlike(globals, &args[1])?;
                Ok(x.log(y).into())
            }),
            NativeFunction::sdnew0(
                "emod",
                &["a", "b"],
                Some("Euclidean modulo on integers"),
                |globals, args, _| {
                    let x = Eval::expect_int(globals, &args[0])?;
                    let y = Eval::expect_int(globals, &args[1])?;
                    Ok(x.rem_euclid(y).into())
                },
            ),
            NativeFunction::sdnew0(
                "ediv",
                &["a", "b"],
                Some("Euclidean division on integers"),
                |globals, args, _| {
                    let x = Eval::expect_int(globals, &args[0])?;
                    let y = Eval::expect_int(globals, &args[1])?;
                    Ok(x.div_euclid(y).into())
                },
            ),
            NativeFunction::sdnew0(
                "fmod",
                &["a", "b"],
                Some("Euclidean modulo on floats"),
                |globals, args, _| {
                    let x = Eval::expect_floatlike(globals, &args[0])?;
                    let y = Eval::expect_floatlike(globals, &args[1])?;
                    Ok(x.rem_euclid(y).into())
                },
            ),
            NativeFunction::sdnew0(
                "fdiv",
                &["a", "b"],
                Some("Euclidean division on floats"),
                |globals, args, _| {
                    let x = Eval::expect_floatlike(globals, &args[0])?;
                    let y = Eval::expect_floatlike(globals, &args[1])?;
                    Ok(x.div_euclid(y).into())
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
