use crate::NativeModule;
use crate::Value;
use std::thread::sleep;
use std::time::Duration;
use std::time::SystemTime;

const NAME: &'static str = "a.time";

pub(super) fn new() -> NativeModule {
    NativeModule::new(NAME, |m| {
        m.doc(concat!("Utility for dealing with time~"));

        m.func(
            "now",
            (),
            concat!("Gets the number of seconds since the unix epoch\n",),
            |_, _, _| {
                let now = SystemTime::now();

                match now.duration_since(SystemTime::UNIX_EPOCH) {
                    Ok(dur) => Ok(dur.as_secs_f64().into()),
                    Err(error) => Ok((-error.duration().as_secs_f64()).into()),
                }
            },
        );

        m.func(
            "sleep",
            ["duration"],
            concat!("Sleeps for the given number of seconds\n",),
            |_, args, _| {
                let nsecs = args.into_iter().next().unwrap().number()?;
                sleep(Duration::from_secs_f64(nsecs));
                Ok(Value::Nil)
            },
        );
    })
}
