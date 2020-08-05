use crate::Globals;
use crate::ReplDelegate;
use std::path::Path;

pub fn main<D: ReplDelegate>(mut globals: Globals, mut repl_delegate: Option<D>) {
    match globals.add_roots_from_env() {
        Ok(()) => (),
        Err(std::env::VarError::NotPresent) => (),
        Err(std::env::VarError::NotUnicode(s)) => {
            eprintln!("Non-unicode MTOTS_PATH variable {:?}", s);
        }
    }
    let all_args: Vec<_> = std::env::args().collect();
    let (interpreter_args, program_args) = {
        let mut interpreter_args: Vec<&str> = Vec::new();
        let mut program_args: Vec<&str> = Vec::new();
        let mut args_iter = all_args.iter().peekable();
        loop {
            if let None | Some("--") = args_iter.peek().map(|s| s.as_ref()) {
                break;
            }
            interpreter_args.push(args_iter.next().unwrap());
        }
        if let Some("--") = args_iter.peek().map(|s| s.as_ref()) {
            args_iter.next().unwrap();
        }
        while let Some(s) = args_iter.next() {
            program_args.push(s);
        }
        (interpreter_args, program_args)
    };

    globals.set_cli_args(program_args.into_iter().map(|s| s.into()).collect());

    // parse the interpreter args
    let mut target = RunTarget::Unspecified;
    let mut state = ArgState::Normal;
    let mut sources = Vec::new();
    let mut repl_requested = false;
    for arg in interpreter_args.into_iter().skip(1) {
        match state {
            ArgState::Normal => match arg {
                "-m" => {
                    state = ArgState::Module;
                }
                "-r" => {
                    repl_requested = true;
                }
                "-h" => {
                    print_help_and_exit(0);
                }
                _ => {
                    sources.push(arg);
                }
            },
            ArgState::Module => {
                target = RunTarget::Module(arg);
                state = ArgState::Normal;
            }
        }
    }
    if let RunTarget::Unspecified = target {
        if repl_requested {
            if let Some(delegate) = &mut repl_delegate {
                target = RunTarget::Repl(delegate);
            } else {
                eprintln!("The REPL is not available in this environment");
                std::process::exit(1);
            }
        } else if let Some(last_source) = sources.pop() {
            target = RunTarget::Path(last_source);
        } else if let Some(delegate) = &mut repl_delegate {
            target = RunTarget::Repl(delegate);
        } else if repl_requested {
            print_help_and_exit(1);
        }
    }

    run(globals, &sources, target);
}

fn print_help_and_exit(code: i32) {
    eprintln!("Usage:");
    eprintln!("  [options..] [sources..] [-- script-args..]");
    eprintln!("");
    eprintln!("options:");
    eprintln!("    -r                request repl (default if no sources are specified)");
    eprintln!("    -m <module-name>  run the specified module");
    std::process::exit(code);
}

enum ArgState {
    Normal,
    Module,
}

enum RunTarget<'a> {
    Path(&'a str),
    Module(&'a str),
    Repl(&'a mut dyn ReplDelegate),
    Unspecified,
}

fn run(mut globals: Globals, extra_sources: &[&str], target: RunTarget) {
    for source in extra_sources {
        add_source(&mut globals, source, None);
    }
    match target {
        RunTarget::Path(path) => {
            add_source(&mut globals, path, Some("__main"));
            globals.exit_on_error(|globals| globals.load_main("__main"));
        }
        RunTarget::Module(module_name) => {
            globals.exit_on_error(|globals| globals.load_main(module_name));
        }
        RunTarget::Repl(delegate) => {
            globals.run_repl(delegate);
        }
        RunTarget::Unspecified => {
            print_help_and_exit(1);
        }
    };
}

fn add_source(globals: &mut Globals, pathstr: &str, name: Option<&str>) {
    let path = Path::new(pathstr);
    if path.is_file() {
        let path_basename = pathstr.split('.').next_back().unwrap();
        let module_name = match name {
            Some(name) => name,
            None => {
                if path_basename.ends_with(".u") {
                    let len = path_basename.len();
                    &path_basename[..len - ".u".len()]
                } else {
                    path_basename
                }
            }
        };
        if let Err(error) = globals.add_file_as_module(module_name.into(), path.into()) {
            eprintln!("Could not read script: {:?}", error);
            std::process::exit(1);
        }
    } else if path.is_dir() {
        // NOTE: in this case the name parameter is ignored
        globals.add_source_root(path.into());
    } else {
        eprintln!("Expected file or directory but got {:?}", path);
        std::process::exit(1);
    }
}
