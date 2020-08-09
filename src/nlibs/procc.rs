use crate::ArgSpec;
use crate::Error;
use crate::Globals;
use crate::NativeModule;
use crate::RcStr;
use crate::Result;
use crate::Value;
use std::convert::TryFrom;
use std::process as pr;

const NAME: &'static str = "a.proc";

pub(super) fn new() -> NativeModule {
    NativeModule::new(NAME, |m| {
        m.doc("Utilities for spawning and interacting with subprocesses");

        m.func(
            "run",
            ArgSpec::builder()
                .req("cmd")
                .def("args", [])
                .def("stdin", "inherit")
                .def("stdout", "inherit")
                .def("stderr", "inherit")
                .def("dir", ())
                .def("encoding", ())
                .def("clear_envs", false)
                .def("envs", ()),
            "",
            |globals, args, _| {
                let mut args = args.into_iter();
                let cmd = args.next().unwrap().into_string()?;
                let mut cmd = pr::Command::new(cmd);
                cmd.args(Vec::<RcStr>::try_from(args.next().unwrap())?);
                cmd.stdin(pr::Stdio::try_from(args.next().unwrap())?);
                cmd.stdout(pr::Stdio::try_from(args.next().unwrap())?);
                cmd.stderr(pr::Stdio::try_from(args.next().unwrap())?);
                match args.next().unwrap() {
                    Value::Nil => {}
                    value => {
                        let dir = value.into_string()?;
                        cmd.current_dir(dir.str());
                    }
                };
                let encoding = Encoding::try_from(args.next().unwrap())?;
                let clear_envs = args.next().unwrap().truthy();
                if clear_envs {
                    cmd.env_clear();
                }
                let envs = args.next().unwrap();
                if !envs.is_nil() {
                    let envs: Vec<(RcStr, RcStr)> = envs.unpack_into(globals)?;
                    cmd.envs(envs);
                }
                let child = cmd.spawn()?;
                let output = child.wait_with_output()?;
                let code = output.status.code().map(Value::from).unwrap_or(Value::Nil);
                let stdout = encoding.decode(globals, output.stdout)?;
                let stderr = encoding.decode(globals, output.stderr)?;
                Ok(Value::from(vec![code, stdout, stderr]))
            },
        );
    })
}

impl TryFrom<Value> for pr::Stdio {
    type Error = Error;
    fn try_from(value: Value) -> Result<pr::Stdio> {
        let opt = match &value {
            Value::String(string) => match string.str() {
                "inherit" => Some(pr::Stdio::inherit()),
                "pipe" => Some(pr::Stdio::piped()),
                "null" => Some(pr::Stdio::null()),
                _ => None,
            },
            _ => None,
        };
        match opt {
            Some(opt) => Ok(opt),
            None => Err(rterr!(
                "Expected 'inherit', 'pipe' or 'null' but got {:?}",
                value
            )),
        }
    }
}

enum Encoding {
    Raw,
    Utf8,
}

impl Encoding {
    pub fn decode(&self, globals: &mut Globals, bytes: Vec<u8>) -> Result<Value> {
        match self {
            Self::Raw => globals.new_handle(bytes).map(Value::from),
            Self::Utf8 => Ok(RcStr::from(std::str::from_utf8(&bytes)?).into()),
        }
    }
}

impl TryFrom<Value> for Encoding {
    type Error = Error;
    fn try_from(value: Value) -> Result<Encoding> {
        let opt = match &value {
            Value::Nil => Some(Encoding::Raw),
            Value::String(string) => match string.str() {
                "raw" => Some(Encoding::Raw),
                "utf8" | "utf-8" => Some(Encoding::Utf8),
                _ => None,
            },
            _ => None,
        };
        opt.ok_or_else(|| {
            rterr!(
                "Expected nil, 'raw', 'utf8', or 'utf-8' but got {:?}",
                value,
            )
        })
    }
}
