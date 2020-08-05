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
            NativeFunction::new("sin", &["x"], None, |globals, args, _| {
                let x = Eval::expect_floatlike(globals, &args[0])?;
                Ok(x.sin().into())
            }),
            NativeFunction::new("cos", &["x"], None, |globals, args, _| {
                let x = Eval::expect_floatlike(globals, &args[0])?;
                Ok(x.cos().into())
            }),
            NativeFunction::new("tan", &["x"], None, |globals, args, _| {
                let x = Eval::expect_floatlike(globals, &args[0])?;
                Ok(x.tan().into())
            }),
            NativeFunction::new("asin", &["x"], None, |globals, args, _| {
                let x = Eval::expect_floatlike(globals, &args[0])?;
                Ok(x.asin().into())
            }),
            NativeFunction::new("acos", &["x"], None, |globals, args, _| {
                let x = Eval::expect_floatlike(globals, &args[0])?;
                Ok(x.acos().into())
            }),
            NativeFunction::new("atan", &["x"], None, |globals, args, _| {
                let x = Eval::expect_floatlike(globals, &args[0])?;
                Ok(x.atan().into())
            }),
            NativeFunction::new("atan2", &["y", "x"], None, |globals, args, _| {
                let y = Eval::expect_floatlike(globals, &args[0])?;
                let x = Eval::expect_floatlike(globals, &args[1])?;
                Ok(y.atan2(x).into())
            }),
            NativeFunction::new("sinh", &["x"], None, |globals, args, _| {
                let x = Eval::expect_floatlike(globals, &args[0])?;
                Ok(x.sinh().into())
            }),
            NativeFunction::new("cosh", &["x"], None, |globals, args, _| {
                let x = Eval::expect_floatlike(globals, &args[0])?;
                Ok(x.cosh().into())
            }),
            NativeFunction::new("tanh", &["x"], None, |globals, args, _| {
                let x = Eval::expect_floatlike(globals, &args[0])?;
                Ok(x.tanh().into())
            }),
            NativeFunction::new("ln", &["x"], None, |globals, args, _| {
                let x = Eval::expect_floatlike(globals, &args[0])?;
                Ok(x.ln().into())
            }),
            NativeFunction::new("log2", &["x"], None, |globals, args, _| {
                let x = Eval::expect_floatlike(globals, &args[0])?;
                Ok(x.log2().into())
            }),
            NativeFunction::new("log10", &["x"], None, |globals, args, _| {
                let x = Eval::expect_floatlike(globals, &args[0])?;
                Ok(x.log10().into())
            }),
            NativeFunction::new("log", &["x", "base"], None, |globals, args, _| {
                let x = Eval::expect_floatlike(globals, &args[0])?;
                let y = Eval::expect_floatlike(globals, &args[1])?;
                Ok(x.log(y).into())
            }),
            NativeFunction::new(
                "emod",
                &["a", "b"],
                "Euclidean modulo on integers",
                |globals, args, _| {
                    let x = Eval::expect_int(globals, &args[0])?;
                    let y = Eval::expect_int(globals, &args[1])?;
                    Ok(x.rem_euclid(y).into())
                },
            ),
            NativeFunction::new(
                "ediv",
                &["a", "b"],
                "Euclidean division on integers",
                |globals, args, _| {
                    let x = Eval::expect_int(globals, &args[0])?;
                    let y = Eval::expect_int(globals, &args[1])?;
                    Ok(x.div_euclid(y).into())
                },
            ),
            NativeFunction::new(
                "fmod",
                &["a", "b"],
                "Euclidean modulo on floats",
                |globals, args, _| {
                    let x = Eval::expect_floatlike(globals, &args[0])?;
                    let y = Eval::expect_floatlike(globals, &args[1])?;
                    Ok(x.rem_euclid(y).into())
                },
            ),
            NativeFunction::new(
                "fdiv",
                &["a", "b"],
                "Euclidean division on floats",
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
