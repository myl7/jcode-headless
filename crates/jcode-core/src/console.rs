//! Console/terminal ANSI capability helpers.

/// Report whether ANSI output is safe to emit on stderr: true exactly when
/// stderr is a terminal.
pub fn stderr_supports_ansi() -> bool {
    use std::io::IsTerminal;
    std::io::stderr().is_terminal()
}
