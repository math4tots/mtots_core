use crate::Key;
use crate::Map;
use crate::NativeModule;
use crate::Value;
use std::convert::TryFrom;
use std::env;

const NAME: &'static str = "a.env";

pub(super) fn new() -> NativeModule {
    NativeModule::new(NAME, |builder| {
        builder
            .doc(concat!(
                "Access to aspects of the process's environment ",
                "(e.g. process arguments, environment variables)",
            ))
            .func(
                "args",
                (),
                concat!(
                    "Returns the command line arguments for the script as a ",
                    "list of strings\n\n",
                    "NOTE: Due to the various ways the command line ",
                    "arguments could be processed by the host program, ",
                    "this function will simply return what the host program ",
                    "passed to the interpreter as what the interpreter's ",
                    "arguments should be.\n",
                    "If the arguments are never explicitly set with the ",
                    "interpreter (with Globals::set_argv), this function ",
                    "will always return nil",
                ),
                |globals, _, _| {
                    Ok(globals
                        .argv()
                        .as_ref()
                        .map(|argv| argv.iter().map(Value::from).collect::<Vec<_>>().into())
                        .unwrap_or(Value::Nil))
                },
            )
            .func(
                "var",
                ["name"],
                concat!(
                    "Gets the environment variable of the given name ",
                    "as a string\n",
                    "Returns nil if the value is not present\n",
                    "Throws an exception if the value is present, ",
                    "but not valid UTF-8\n",
                ),
                |_globals, args, _| {
                    env::var_os(args.into_iter().next().unwrap().string()?)
                        .map(Value::try_from)
                        .unwrap_or(Ok(Value::Nil))
                },
            )
            .func(
                "vars",
                (),
                concat!(
                    "Gets the environment variable of the given name\n",
                    "Throws an exception if the value is not valid UTF-8\n",
                ),
                |_globals, _, _| {
                    Ok(env::vars()
                        .map(|(key, val)| (Key::from(key), Value::from(val)))
                        .collect::<Map>()
                        .into())
                },
            )
            .func(
                "home",
                (),
                concat!(
                    "Makes a best guess at the home directory\n",
                    "On windows his will return 'UserProfile' environment variable, ",
                    "and everywhere else it will return the 'HOME' environment ",
                    "variable\n",
                ),
                |_, _, _| {
                    let var = if env::consts::OS == "windows" {
                        env::var("UserProfile")
                    } else {
                        env::var("HOME")
                    };
                    match var {
                        Ok(value) => Ok(Value::from(value)),
                        _ => Ok(Value::Nil),
                    }
                },
            )
            .build()
    })
}
