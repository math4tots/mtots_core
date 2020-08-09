mod annotator;
mod ast;
mod backend;
mod compiler;
mod er;
mod frontend;
mod gl;
mod value;

pub use annotator::*;
pub use ast::*;
pub use backend::*;
pub use compiler::*;
pub use er::*;
pub use frontend::*;
pub use gl::*;
pub use value::*;

#[cfg(feature = "line")]
pub(crate) fn mtots_home() -> Option<std::path::PathBuf> {
    crate::home().and_then(|path| {
        let path = std::path::PathBuf::from(path).join(".mtots");
        match std::fs::create_dir_all(&path) {
            Ok(_) => Some(path),
            Err(_) => None,
        }
    })
}

#[cfg(feature = "line")]
pub(crate) fn mtots_line_history_path() -> Option<std::path::PathBuf> {
    mtots_home().map(|home| home.join("line_history"))
}
