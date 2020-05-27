use crate::Globals;
use std::path::Path;

pub fn main(globals: &mut Globals) {
    match globals.add_roots_from_env() {
        Ok(()) => (),
        Err(std::env::VarError::NotPresent) => (),
        Err(std::env::VarError::NotUnicode(s)) => {
            eprintln!("Non-unicode MTOTS_PATH variable {:?}", s);
        }
    }
    let all_args: Vec<_> = std::env::args().collect();
    let (interpreter_args, _program_args) = {
        let mut interpreter_args: Vec<&str> = Vec::new();
        let mut program_args: Vec<&str> = Vec::new();
        let mut args_iter = all_args.iter().peekable();
        loop {
            if let None | Some("--") = args_iter.peek().map(|s| s.as_ref()) {
                break;
            }
            interpreter_args.push(args_iter.next().unwrap());
        }
        while let Some(s) = args_iter.next() {
            program_args.push(s);
        }
        (interpreter_args, program_args)
    };

    match interpreter_args.as_slice() {
        &[_, path] => {
            let path = Path::new(path);
            if path.is_file() {
                if let Err(error) = globals.add_file_as_module("__main".into(), path.into()) {
                    eprintln!("Could not read script: {:?}", error);
                    std::process::exit(1);
                }
            } else if path.is_dir() {
                globals.add_source_root(path.into());
            } else {
                eprintln!("Expected file or directory but got {:?}", path);
                std::process::exit(1);
            }
            if let Err(_) = globals.load_main() {
                let error = globals.exc_move();
                eprint!("{}\n{}", error, globals.trace_fmt());
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!("<path-to-script> [-- args...]");
        }
    }
}