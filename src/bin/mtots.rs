extern crate mtots_core;
use mtots_core::Value;
use mtots_core::Result;
use mtots_core::Globals;
use mtots_core::RcStr;
use std::path::Path;
use std::io::Write;

fn main() {
    let mut source_roots = Vec::new();
    let mut mode = Mode::Normal;
    let mut command = Command::Unspecified;

    for argstr in std::env::args().skip(1) {
        let arg: &str = &argstr;
        match mode {
            Mode::Normal => match arg {
                "-m" => mode = Mode::SetRunModule,
                "-r" => command = Command::Repl,
                _ => {
                    let path = Path::new(arg);
                    if !path.exists() {
                        eprintln!("Path {:?} does not exist", path);
                        std::process::exit(1);
                    }
                    source_roots.push(argstr);
                }
            },
            Mode::SetRunModule => {
                command = Command::RunModule(argstr);
                mode = Mode::Normal;
            }
        }
    }

    if let Command::Unspecified = command {
        command = if let Some(path) = source_roots.pop() {
            Command::RunPath(path)
        } else {
            Command::Repl
        };
    }

    let mut globals = mtots_core::Globals::new();
    for source_root in source_roots {
        globals.add(source_root).unwrap();
    }

    match command {
        Command::Unspecified => panic!("Command::Unspecified should be unreachable"),
        Command::Repl => repl(globals),
        Command::RunModule(module) => run_module(globals, &module.into()),
        Command::RunPath(pathstr) => run_path(globals, pathstr),
    }
}

enum Mode {
    Normal,
    SetRunModule,
}

enum Command {
    Unspecified,
    Repl,
    RunModule(String),
    RunPath(String),
}

fn print(msg: &str) {
    print!("{}", msg);
    std::io::stdout().flush().unwrap();
}

fn repl(mut globals: Globals) {
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

fn run_module(mut globals: Globals, module: &RcStr) {
    let r = globals.load(module).map(|_| ());
    ordie(&mut globals, r);
}

fn run_path(mut globals: Globals, pathstr: String) {
    let path = Path::new(&pathstr);
    if path.is_dir() {
        let r = globals.add(pathstr);
        ordie(&mut globals, r);
        run_module(globals, &"__main".into());
    } else {
        let data = std::fs::read_to_string(path).unwrap();
        let r = globals.exec_str("__main", &data);
        ordie(&mut globals, r);
    }
}

fn ordie<T>(globals: &mut Globals, r: Result<T>) -> T {
    match r {
        Ok(t) => t,
        Err(error) => {
            let error = error.prepended(globals.trace().clone());
            eprint!("{}", error.format());
            std::process::exit(1);
        }
    }
}
