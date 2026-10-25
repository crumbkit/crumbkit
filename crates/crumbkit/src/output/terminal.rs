//! `Output::progress_allowed`, which decides whether a tool may draw
//! progress from `--json`, the guide's terminal detection and standard
//! error (ADR-0049), and the width of the terminal that a text table fits
//! (ADR-0121), and what decides whether a refusal on standard error and a
//! table's cells on standard output are styled (ADR-0131, ADR-0150).

use clap::ColorChoice;

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
            self.is_json(),
            !self.no_terminal,
            crate::sink::stderr_is_terminal,
        )
    }

    /// Returns the number of columns of the terminal that standard output
    /// is, which `finish` fits a text table to, or `None` when the guide
    /// turns terminal detection off, standard output is not a terminal, or
    /// the terminal does not report its width.
    pub(super) fn read_terminal_width(&self) -> Option<usize> {
        width(!self.no_terminal, crate::sink::stdout_width)
    }

    /// Reports whether the cells of a text table that `finish` prints
    /// carry their styles: the guide keeps terminal detection on, standard
    /// output is a terminal, and the colour variables and the command's
    /// `ColorChoice` allow colour, as for a refusal (ADR-0150).
    pub(super) fn read_result_styles(&self) -> bool {
        crate::sink::stdout_styles(self.command.get_color(), !self.no_terminal)
    }

    /// Returns the command's `ColorChoice` and whether the guide keeps
    /// terminal detection on, which decide with the environment and
    /// standard error whether a refusal on standard error is styled
    /// (ADR-0131).
    pub(crate) fn refusal_colour(&self) -> (ColorChoice, bool) {
        (self.command.get_color(), !self.no_terminal)
    }
}

/// Reports whether progress is allowed: `json` is off, `detection` is on
/// and `terminal` reports a terminal. `terminal` runs only when the other
/// two allow progress, so a run without detection asks about no stream.
fn allowed(json: bool, detection: bool, terminal: impl FnOnce() -> bool) -> bool {
    !json && detection && terminal()
}

/// Returns the width that `read` returns when `detection` is on, and `None`
/// otherwise. `read` runs only with detection on, so a run without
/// detection asks about no stream.
fn width(detection: bool, read: impl FnOnce() -> Option<usize>) -> Option<usize> {
    if detection { read() } else { None }
}
