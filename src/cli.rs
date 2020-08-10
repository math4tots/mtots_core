use crate::Globals;
use crate::RcStr;
use crate::Result;
use crate::Value;
use std::path::Path;

pub fn climain(mut globals: Globals) {
    let mut source_roots = Vec::new();
    let mut script_args = Vec::new();
    let mut mode = Mode::Normal;
    let mut command = Command::Unspecified;

    for argstr in std::env::args().skip(1) {
        let arg: &str = &argstr;
        match mode {
            Mode::Normal => match arg {
                "-m" => mode = Mode::SetRunModule,
                "-d" => mode = Mode::SetDocModule,
                "-r" => command = Command::Repl,
                "--" => mode = Mode::ScriptArgs,
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
            Mode::ScriptArgs => {
                script_args.push(RcStr::from(argstr));
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

    globals.set_argv(script_args);
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
    ScriptArgs,
}

enum Command {
    Unspecified,
    Repl,
    DocModule(String),
    RunModule(String),
    RunPath(String),
}

fn repl(mut globals: Globals) {
    let trace_base = globals.trace().len();
    'exit: loop {
        let mut input = globals.readline(">> ").unwrap();
        let mut line = String::new();
        loop {
            match input {
                None => break 'exit,
                Some(part) => {
                    line.push_str(&part);
                }
            }
            if globals.repl_ready(&line) {
                break;
            }
            input = globals.readline(".. ").unwrap();
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
        globals.save_line_history().unwrap();
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
        println!("\n{}", format_doc(doc, 0));
    }
    println!("Members");
    println!("=======");
    let mut pairs: Vec<_> = module.docmap().iter().collect();
    pairs.sort();
    for (field_name, field_doc) in pairs {
        print!("  ");
        let value = match module.map().get(field_name) {
            Some(value) => value.borrow().clone(),
            None => panic!(
                "{}: Doc for field {:?} found with no associated value",
                module.name(),
                field_name
            ),
        };
        match value {
            Value::Function(func) => {
                let type_ = if func.is_generator() { "def*" } else { "def" };
                println!("{} {}{}\n", type_, field_name, func.argspec());
            }
            Value::NativeFunction(func) => {
                println!("native def {}{}\n", field_name, func.argspec());
            }
            value if short_printable_value(&value) => {
                println!("{} = {:?}\n", field_name, value);
            }
            _ => println!("{}", field_name),
        }
        println!("{}", format_doc(field_doc, 8));
    }
}

fn short_printable_value(value: &Value) -> bool {
    match value {
        Value::Nil | Value::Bool(_) | Value::Number(_) => true,
        Value::String(s) => {
            s.len() < 40 && s.chars().all(|c| !c.is_control() && c != '\n' && c != '\t')
        }
        _ => false,
    }
}

fn format_doc(doc: &str, indent: usize) -> String {
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
                    .chunks(LINE_WIDTH - indent)
                    .map(|chars| chars.iter().collect::<String>())
                    .collect::<Vec<_>>()
            }
        })
        .map(|line| format!("{}{}\n", " ".repeat(indent), line))
        .collect::<Vec<_>>()
        .join("")
}

fn run_module(mut globals: Globals, module: &RcStr) {
    globals.set_main(module.clone());
    let r = globals.load(module).map(|_| ());
    ordie(&mut globals, r);
    globals.handle_trampoline();
}

fn run_path(mut globals: Globals, pathstr: String) {
    let path = Path::new(&pathstr);
    if path.is_dir() {
        let r = globals.add(pathstr);
        ordie(&mut globals, r);
        run_module(globals, &"__main".into());
    } else {
        let data = std::fs::read_to_string(path).unwrap();
        globals.set_main("__main".into());
        let r = globals.exec_str("__main", Some(&pathstr), &data);
        ordie(&mut globals, r);
        globals.handle_trampoline();
    }
}

pub fn ordie<T>(globals: &mut Globals, r: Result<T>) -> T {
    match r {
        Ok(t) => t,
        Err(error) => {
            let error = error.prepended(globals.trace().clone());
            eprint!("{}", error.format());
            std::process::exit(1);
        }
    }
}
