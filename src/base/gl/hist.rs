use super::*;

/// Functions for readline-style editing and history
/// These functions only work if the "line" feature of this crate is enabled.
/// Otherwise these will be nops (for history related) and a much more
/// primitive implementation (for the readline method).
impl Globals {
    #[cfg(feature = "line")]
    pub(super) fn new_line_editor() -> rustyline::Editor<()> {
        let mut editor = rustyline::Editor::<()>::with_config({
            rustyline::Config::builder().auto_add_history(true).build()
        });
        if let Some(history_path) = crate::mtots_line_history_path() {
            editor.load_history(&history_path).unwrap_or(());
        }
        editor
    }

    /// Read a line of input
    /// Analogous to Python's 'input' function
    /// Uses rustyline if the line feature is enabled
    #[cfg(feature = "line")]
    pub fn readline(&mut self, prompt: &str) -> Result<Option<String>> {
        match self.line.readline(prompt) {
            Ok(line) => Ok(Some(line)),
            Err(rustyline::error::ReadlineError::Eof) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
    /// Read a line of input
    /// Analogous to Python's 'input' function
    /// Uses rustyline if the line feature is enabled
    #[cfg(not(feature = "line"))]
    pub fn readline(&mut self, prompt: &str) -> Result<Option<String>> {
        use std::io::Write;
        print!("{}", prompt);
        std::io::stdout().flush().unwrap();
        let mut buf = String::new();
        let len = std::io::stdin().read_line(&mut buf)?;
        Ok(if len == 0 { None } else { Some(buf) })
    }
    #[cfg(feature = "line")]
    pub fn save_line_history(&mut self) -> Result<()> {
        if let Some(path) = crate::mtots_line_history_path() {
            self.line.save_history(&path)?;
        }
        Ok(())
    }
    #[cfg(not(feature = "line"))]
    pub fn save_line_history(&mut self) -> Result<()> {
        Ok(())
    }
}
