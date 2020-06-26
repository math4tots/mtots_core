extern crate mtots_core;
use std::io::Write;
use mtots_core::ReplDelegate;

fn main() {
    let globals = mtots_core::Globals::new();
    mtots_core::main(globals, Some(Delegate));
}

struct Delegate;

impl ReplDelegate for Delegate {
    fn getline(&mut self, continuation: bool) -> Option<String> {
        if continuation {
            print!(".. ");
        } else {
            print!(">> ");
        }
        match std::io::stdout().flush() {
            Ok(_) => {},
            Err(_) => return None,
        }
        let mut line = String::new();
        match std::io::stdin().read_line(&mut line) {
            Ok(len) => if len == 0 { None } else { Some(line) },
            Err(_) => None,
        }
    }
}
