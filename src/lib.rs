/// try for generators (where '?' operator won't work)
#[macro_export]
macro_rules! gentry {
    ($e:expr) => {
        match $e {
            Ok(t) => t,
            Err(error) => return crate::ResumeResult::Err(crate::Error::from(error)),
        }
    };
}

mod base;
mod nlibs;
mod util;

// I feel really yucky depending on an external crate in core
// Eventually I might just replace this with a makeshift implementation
// and eat the performance costs
pub extern crate indexmap;

pub use base::*;
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
