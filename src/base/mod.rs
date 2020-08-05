mod ast;
mod code;
mod compiler;
mod emb;
mod entry;
mod eval;
mod frontend;
mod globals;
mod repl;
mod value;

pub use ast::ArgumentList;
pub use ast::Binop;
pub use ast::Expression;
pub use ast::ExpressionData;
pub use ast::ExpressionKind;
pub use ast::Operation;
pub use ast::Unop;
pub use code::Code;
pub use code::CodeBuilder;
pub use code::CodeBuilderError;
pub use code::CodeKind;
pub use code::Frame;
pub use code::FrameError;
pub use code::GeneratorResult;
pub use compiler::compile;
pub use compiler::CompileError;
pub use compiler::CompileErrorKind;
pub use entry::main;
pub use eval::Eval;
pub use eval::EvalError;
pub use eval::EvalResult;
pub use eval::VMap;
pub use eval::VSet;
pub use frontend::LexError;
pub use frontend::LexErrorKind;
pub use frontend::Lexer;
pub use frontend::ParameterKind;
pub use frontend::ParseError;
pub use frontend::ParseErrorKind;
pub use frontend::Parser;
pub use frontend::Punctuator;
pub use frontend::Token;
pub use frontend::TokenKind;
pub use globals::BuiltinClasses;
pub use globals::BuiltinExceptions;
pub use globals::ErrorIndicator;
pub use globals::Exception;
pub use globals::ExceptionKind;
pub use globals::Globals;
pub use globals::NativeFunctions;
pub use globals::Stashable;
pub use globals::SOURCE_FILE_EXTENSION;
pub(crate) use repl::run_repl;
pub use repl::ReplDelegate;
pub use repl::ReplScope;
pub use value::*;

use crate::RcPath;
use crate::RcStr;
use std::fmt;

pub enum SourceName {
    File(RcPath),
    ModuleName(RcStr),
}

impl fmt::Display for SourceName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SourceName::File(path) => write!(f, "File {:?}", path),
            SourceName::ModuleName(name) => write!(f, "Module {}", name),
        }
    }
}

pub fn short_name_from_full_name(full_name: &RcStr) -> RcStr {
    match full_name.rfind("::") {
        Some(i) => full_name[i + "::".len()..].into(),
        None => match full_name.rfind(".") {
            Some(i) => full_name[i + 1..].into(),
            None => full_name.clone(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Symbol;

    #[test]
    fn print_through_bytecode() {
        use crate::RcStr;

        let mut globals = Globals::new();
        let mut builder = CodeBuilder::for_module("<test>".into(), None);
        let message: RcStr = "hello world!".into();

        builder.load_const(message.clone());
        builder.dup_top();
        builder.store_var("foo".into());

        // builder.load_var("print".into());
        // builder.load_const(message);
        // builder.call_func(1);

        let code = builder.build().unwrap();
        let (mut frame, module) = Frame::for_module(&code, None, globals.builtins()).unwrap();
        code.run(&mut globals, &mut frame).unwrap();

        let key = Symbol::from("foo");
        assert_eq!(
            &**module.get(&key).unwrap().string().unwrap(),
            "hello world!",
        );
    }

    #[test]
    fn sample_exec() {
        let mut globals = Globals::new();
        globals
            .exec_module(
                "<test>".into(),
                None,
                r####"
                print("Hello world!")
                print("Hello world again")
                "####
                    .into(),
            )
            .unwrap();
    }

    #[test]
    fn code_debugstr() {
        let mut globals = Globals::new();
        let expr = globals
            .parse(
                "<test>".into(),
                r###"
        print("Hello world!")
        print("Hello world again")
        "###,
            )
            .unwrap();
        let code = compile("<test>".into(), &expr).unwrap();
        let dstr = code.debugstr0();
        assert_eq!(
            dstr,
            r##"Code object <test>
     2      0 LOAD_DEREF 0 (print (free))
            2 LOAD_CONST 0 (String("Hello world!"))
            4 CALL_FUNCTION 2, 1
            7 POP_TOP
     3      8 LOAD_DEREF 0 (print (free))
           10 LOAD_CONST 1 (String("Hello world again"))
           12 CALL_FUNCTION 3, 1
"##
        );
    }

    #[test]
    fn value_and_eval_result_size() {
        use std::mem::size_of;

        assert_eq!(size_of::<Value>(), 2 * size_of::<usize>());
        assert_eq!(size_of::<EvalResult<Value>>(), 2 * size_of::<usize>());

        // The error indicator itself requires no data, so EvalResult<()> should
        // not need more than one byte
        assert_eq!(size_of::<EvalResult<()>>(), 1);
    }
}
