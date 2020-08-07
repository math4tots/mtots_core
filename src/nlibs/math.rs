use crate::ArgSpec;
use crate::Globals;
use crate::NativeModule;
use crate::RcStr;
use crate::Result;
use crate::Value;
use std::collections::HashMap;

pub(super) fn new() -> NativeModule {
    NativeModule::new("a.math", |builder| {
        builder
            .val("PI", "", std::f64::consts::PI)
            .val("TAU", "", std::f64::consts::PI * 2.0)
            .val("E", "", std::f64::consts::E)
            .func("sin", ["x"], "Computes sin in radians", wrap1(f64::sin))
            .func("cos", ["x"], "Computes cos in radians", wrap1(f64::cos))
            .func("tan", ["x"], "Computes tan in radians", wrap1(f64::tan))
            .func("asin", ["x"], "Computes asin in radians", wrap1(f64::asin))
            .func("acos", ["x"], "Computes acos in radians", wrap1(f64::acos))
            .func("atan", ["x"], "Computes atan in radians", wrap1(f64::atan))
            .func(
                "atan2",
                ["y", "x"],
                "Computes atan2 in radians",
                wrap2(f64::atan2),
            )
            .func("sinh", ["x"], "Computes sinh in radians", wrap1(f64::sinh))
            .func("cosh", ["x"], "Computes cosh in radians", wrap1(f64::cosh))
            .func("tanh", ["x"], "Computes tanh in radians", wrap1(f64::tanh))
            .func(
                "log",
                ArgSpec::builder().req("x").def("base", std::f64::consts::E),
                "Computes the logarithm.\nIf not specified, the base defaults to e",
                wrap2(f64::log),
            )
            .func("log10", ["x"], "Computes log base 10", wrap1(f64::log10))
            .func("abs", ["x"], "Absolute value", wrap1(f64::abs))
            .func(
                "sqrt",
                ["x"],
                "Returns the square root of a number",
                wrap1(f64::sqrt),
            )
            .func(
                "ln",
                ["x"],
                "Computes the natural logarithm",
                wrap1(f64::ln),
            )
            .func("log2", ["x"], "Computes log base 2", wrap1(f64::log2))
            .build()
    })
}

fn wrap1<F: Fn(f64) -> f64 + 'static>(
    f: F,
) -> impl Fn(&mut Globals, Vec<Value>, Option<HashMap<RcStr, Value>>) -> Result<Value> + 'static {
    move |_, args, _| {
        let x = args.into_iter().next().unwrap().number()?;
        Ok(Value::from(f(x)))
    }
}

fn wrap2<F: Fn(f64, f64) -> f64 + 'static>(
    f: F,
) -> impl Fn(&mut Globals, Vec<Value>, Option<HashMap<RcStr, Value>>) -> Result<Value> + 'static {
    move |_, args, _| {
        let mut args = args.into_iter();
        let a = args.next().unwrap().number()?;
        let b = args.next().unwrap().number()?;
        Ok(Value::from(f(a, b)))
    }
}
