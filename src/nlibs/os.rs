use crate::NativeModule;

const NAME: &'static str = "a.os";

pub(super) fn new() -> NativeModule {
    NativeModule::new(NAME, |builder| {
        builder
            .doc(concat!(
                "Some information about the host environment and operating system",
            ))
            .val(
                "arch",
                concat!(
                    "From Rust's std::env::consts::ARCH\n\n",
                    "A string describing the architecture of the CPU ",
                    "that is currently in use.\n",
                    "Some possible values:\n",
                    "    * x86\n",
                    "    * x86_64\n",
                    "    * arm\n",
                    "    * aarch64\n",
                    "    * mips\n",
                    "    * mips64\n",
                    "    * powerpc\n",
                    "    * powerpc64\n",
                    "    * riscv64\n",
                    "    * s390x\n",
                    "    * sparc64\n",
                ),
                std::env::consts::FAMILY,
            )
            .val(
                "family",
                concat!(
                    "From Rust's std::env::consts::FAMILY\n\n",
                    "The family of the operating system. Example value is unix.\n",
                    "Some possible values:\n",
                    "    * unix,\n",
                    "    * windows\n",
                ),
                std::env::consts::FAMILY,
            )
            .val(
                "name",
                concat!(
                    "From Rust's std::env::consts::OS\n\n",
                    "A string describing the specific operating system in use. ",
                    "Example value is linux.\n",
                    "Some possible values:\n",
                    "    * linux\n",
                    "    * macos\n",
                    "    * ios\n",
                    "    * freebsd\n",
                    "    * dragonfly\n",
                    "    * netbsd\n",
                    "    * openbsd\n",
                    "    * solaris\n",
                    "    * android\n",
                    "    * windows\n",
                ),
                std::env::consts::FAMILY,
            );
    })
}
