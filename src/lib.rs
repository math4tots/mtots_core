/// try for generators (where '?' operator won't work)
#[macro_export]
macro_rules! gentry {
    ($e:expr) => {
        match $e {
            Ok(t) => t,
            Err(error) => return $crate::ResumeResult::Err($crate::Error::from(error)),
        }
    };
}
/// Outside of mtots_core, you can't just impl From<..> for mtots_core::Error
/// This macro is for conveniently converting an error of any type that
/// already implements std::error::Error, into mtots_core::Error
#[macro_export]
macro_rules! mtry {
    ($e:expr) => {
        match $e {
            Ok(t) => t,
            Err(error) => return Err($crate::Error::from_std(error)),
        }
    };
}

/// Utility for constructing runtime errors
#[macro_export]
macro_rules! rterr {
    ( $($args:expr),+ $(,)?) => {
        $crate::Error::rt(
            format!( $($args),+ ).into(),
            vec![])
    };
}

mod base;
mod cli;
mod nlibs;
mod util;

// I feel really yucky depending on an external crate in core
// Eventually I might just replace this with a makeshift implementation
// and eat the performance costs
pub extern crate indexmap;

pub use base::*;
pub use cli::climain;
pub use cli::ordie;
pub use indexmap::IndexMap;
pub use indexmap::IndexSet;
pub use nlibs::*;
pub use util::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello() {
        let mut globals = Globals::new();
        globals
            .exec_str(
                "[test]",
                None,
                r###"
                print("Hello world")
                "###,
            )
            .unwrap();
    }
}
