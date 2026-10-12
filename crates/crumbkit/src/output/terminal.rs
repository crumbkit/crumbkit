//! `Output::progress_allowed`, which decides whether a tool may draw
//! progress from `--json`, the guide's terminal detection and standard
//! error (ADR-0049).

use super::Output;

#[cfg(test)]
mod tests;

impl Output {
    /// Reports whether the tool may draw progress, such as a spinner or a
    /// bar, on standard error. It is true only when `--json` is off, the
    /// tool's guide keeps terminal detection on
    /// ([`Guide::without_terminal_detection`](crate::help::Guide::without_terminal_detection)),
    /// and standard error is a terminal.
    ///
    /// A tool draws progress with any library it likes when this is true,
    /// and prints nothing extra otherwise, so a program or an agent that
    /// reads the output, and a pipe or a file, receive no progress lines.
    /// `check_examples` in crumbkit-test runs each example with standard
    /// error piped, so an example never draws progress.
    #[must_use]
    pub fn progress_allowed(&self) -> bool {
        allowed(
            self.json,
            !self.no_terminal,
            crate::sink::stderr_is_terminal,
        )
    }
}

/// Reports whether progress is allowed: `json` is off, `detection` is on
/// and `terminal` reports a terminal. `terminal` runs only when the other
/// two allow progress, so a run without detection asks about no stream.
fn allowed(json: bool, detection: bool, terminal: impl FnOnce() -> bool) -> bool {
    !json && detection && terminal()
}
