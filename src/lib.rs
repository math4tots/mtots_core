mod base;
mod util;

pub use base::*;
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
                r###"
                print("Hello world")
                "###,
            )
            .unwrap();
    }
}
