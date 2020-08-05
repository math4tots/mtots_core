mod base;
mod util;

// I feel really yucky depending on an external crate in core
// Eventually I might just replace this with a makeshift implementation
// and eat the performance costs
pub extern crate indexmap;

pub use base::*;
pub use indexmap::IndexMap;
pub use indexmap::IndexSet;
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
