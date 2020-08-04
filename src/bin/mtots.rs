extern crate mtots_core;
use mtots_core::Value;
use std::io::Write;

fn main() {
    let mut globals = mtots_core::Globals::new();
    let trace_base = globals.trace().len();
    'exit: loop {
        let mut line = String::new();
        print(">> ");
        loop {
            match std::io::stdin().read_line(&mut line) {
                Ok(0) => break 'exit,
                Ok(_) => {}
                Err(error) => {
                    panic!("{:?}", error);
                }
            }
            if globals.repl_ready(&line) {
                break;
            }
            print(".. ");
        }
        match globals.exec_repl(&line) {
            Ok(Value::Nil) => {}
            Ok(value) => {
                println!("{}", value);
            }
            Err(error) => {
                let error = error.prepended(globals.trace().clone());
                eprint!("{}", error.format());
                globals.trace_unwind(trace_base);
            }
        }
    }
}

fn print(msg: &str) {
    print!("{}", msg);
    std::io::stdout().flush().unwrap();
}
