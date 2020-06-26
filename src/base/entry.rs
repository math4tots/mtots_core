use crate::ReplDelegate;
use crate::Globals;
use std::path::Path;

pub fn main<D: ReplDelegate>(mut globals: Globals, repl_delegate: Option<D>) {
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

    match interpreter_args.as_slice() {
        &[_] if repl_delegate.is_some() => {
            run(globals, &[], RunTarget::Repl(&mut repl_delegate.unwrap()));
        }
        &[_, path] => {
            run(globals, &[], RunTarget::Path(&path));
        }
        &[_, extra_source, "-m", module_name] => {
            run(globals, &[extra_source], RunTarget::Module(module_name));
        }
        &[_, "-m", module_name] => {
            run(globals, &[], RunTarget::Module(module_name));
        }
        &[_, extra_source, path] => {
            run(globals, &[extra_source], RunTarget::Path(&path));
        }
        _ => {
            eprintln!("<path-to-script> [-- args...]");
            std::process::exit(1)
        }
    }
}

enum RunTarget<'a> {
    Path(&'a str),
    Module(&'a str),
    Repl(&'a mut dyn ReplDelegate),
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
