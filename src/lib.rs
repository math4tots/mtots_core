mod base;
mod util;

use base::compile;
use base::run_repl;
use base::short_name_from_full_name;
use base::ArgumentError;
use base::ArgumentList;
use base::Binop;
use base::Code;
use base::CodeBuilder;
use base::CodeBuilderError;
use base::CodeKind;
use base::CompileErrorKind;
use base::ConstValue;
use base::Expression;
use base::ExpressionData;
use base::ExpressionKind;
use base::Frame;
use base::FrameError;
use base::LexError;
use base::LexErrorKind;
use base::Lexer;
use base::Operation;
use base::ParseErrorKind;
use base::Parser;
use base::Punctuator;
use base::SourceName;
use base::Token;
use base::TokenKind;
use base::Unop;
use util::divmod;
use util::gsort;
use util::FailableEq;
use util::FailableHash;
use util::UnorderedHasher;

pub use base::SOURCE_FILE_EXTENSION;
pub use base::*;
pub use util::GMap;
pub use util::HMap;
pub use util::RcPath;
pub use util::RcStr;
pub use util::Symbol;
pub use util::SymbolRegistryHandle;

/// When running in REPL mode, this is what we pretend the module name is
pub const REPL_PSEUDO_MODULE_NAME: &'static str = "[repl]";
