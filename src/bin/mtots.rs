extern crate mtots_core;
use mtots_core::Globals;
use mtots_core::RcStr;
use mtots_core::Result;
use mtots_core::Value;
use std::io::Write;
use std::path::Path;

fn main() {
    let mut source_roots = Vec::new();
    let mut mode = Mode::Normal;
    let mut command = Command::Unspecified;

    for argstr in std::env::args().skip(1) {
        let arg: &str = &argstr;
        match mode {
            Mode::Normal => match arg {
                "-m" => mode = Mode::SetRunModule,
                "-d" => mode = Mode::SetDocModule,
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
            Mode::SetDocModule => {
                command = Command::DocModule(argstr);
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
        Command::DocModule(module) => doc_module(globals, &module.into()),
        Command::RunModule(module) => run_module(globals, &module.into()),
        Command::RunPath(pathstr) => run_path(globals, pathstr),
    }
}

enum Mode {
    Normal,
    SetRunModule,
    SetDocModule,
}

enum Command {
    Unspecified,
    Repl,
    DocModule(String),
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

fn doc_module(mut globals: Globals, module: &RcStr) {
    globals.set_main(module.clone());
    let r = globals.load(module).map(|m| m.clone());
    let module = ordie(&mut globals, r);
    let header = format!("Module {}", module.name());
    println!("{}", header);
    for _ in header.chars() {
        print!("=");
    }
    println!("");
    if let Some(doc) = module.doc() {
        println!("\n{}", doc.trim());
    }
    println!("");
    println!("Members");
    println!("=======");
    let mut pairs: Vec<_> = module.docmap().iter().collect();
    pairs.sort();
    for (field_name, field_doc) in pairs {
        match module.map().get(field_name).map(|f| f.borrow().clone()) {
            Some(Value::Function(func)) => {
                let type_ = if func.is_generator() { "def*" } else { "def" };
                println!("  {} {}{}\n", type_, field_name, func.argspec());
            }
            Some(Value::NativeFunction(func)) => {
                println!("  native def {}{}\n", field_name, func.argspec());
            }
            _ => {
                println!("  {}", field_name);
            }
        }
        println!("{}", format_field_doc(field_doc));
    }
}

fn format_field_doc(doc: &str) -> String {
    const LINE_WIDTH: usize = 80;
    doc.lines()
        .flat_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                vec!["".to_owned()]
            } else {
                line.trim()
                    .chars()
                    .collect::<Vec<_>>()
                    .chunks(LINE_WIDTH)
                    .map(|chars| chars.iter().collect::<String>())
                    .collect::<Vec<_>>()
            }
        })
        .map(|line| format!("        {}\n", line))
        .collect::<Vec<_>>()
        .join("")
}

fn run_module(mut globals: Globals, module: &RcStr) {
    globals.set_main(module.clone());
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
        let r = globals.exec_str("__main", Some(&pathstr), &data);
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
