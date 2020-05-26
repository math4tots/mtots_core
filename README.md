# mtots_core

Core of the mtots scripting language.
This package should have zero external dependencies.

You can get started with running the interpreter with `cargo run`.

See `samples/` for sample scripts

`MTOTS_PATH` works the way `PYTHONPATH` does for Python.

## testing

In addition to `cargo test`, I like to run tests written in mtots itself by running
`cargo run -- ./samples/tests`
