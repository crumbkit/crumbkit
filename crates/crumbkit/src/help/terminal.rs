//! The guide's terminal detection setting, which decides whether crumbkit
//! asks whether a stream is a terminal (ADR-0049).

use super::Guide;

#[cfg(test)]
mod tests;

impl Guide {
    /// Turns terminal detection off, so crumbkit never asks whether a
    /// stream is a terminal and runs no agent detection for a question.
    /// `reason` says why the tool needs it, and crumbkit-test's opt-out
    /// report shows it under the rule `terminal-detection/off` with its
    /// costs (ADR-0036).
    ///
    /// Terminal detection changes only help colour, progress, questions and
    /// the width of a text table, and never another part of a result, a
    /// failure or a note (ADR-0049, ADR-0121). With it off:
    ///
    /// - [`parse`](crate::parse) prints help and the version as plain text,
    ///   also when another crate in the build turns on clap's `color`
    ///   feature.
    /// - [`Output::progress_allowed`](crate::Output::progress_allowed) is
    ///   false.
    /// - The guide's [picker](Guide::picker) never runs, and `parse`
    ///   refuses a command line that leaves out a required argument as it
    ///   does without a picker.
    /// - [`Output::confirm`](crate::Output::confirm) and
    ///   [`Output::confirm_to`](crate::Output::confirm_to) never ask, and
    ///   fail with `<tool>::confirmation::needs_yes` as they do with
    ///   `--no-input`.
    /// - [`finish`](crate::finish) never fits a text table to the
    ///   terminal's width, so a table prints the bytes that it prints
    ///   through a pipe.
    ///
    /// The [`Output`](crate::Output) that `parse` returns copies the setting
    /// from the guide, and so does the one that
    /// [`Output::new`](crate::Output::new) builds for a tool that parses
    /// with clap alone.
    ///
    /// ```
    /// use crumbkit::help::Guide;
    ///
    /// const GUIDE: Guide =
    ///     Guide::new().without_terminal_detection("Our CI runs in a pseudo-terminal.");
    /// assert_eq!(
    ///     GUIDE.get_without_terminal_detection(),
    ///     Some("Our CI runs in a pseudo-terminal.")
    /// );
    /// ```
    #[must_use]
    pub const fn without_terminal_detection(mut self, reason: &'static str) -> Self {
        self.without_terminal_detection = Some(reason);
        self
    }

    /// Returns the reason that [`Guide::without_terminal_detection`] set,
    /// or `None` when crumbkit detects terminals for this guide's tool. The
    /// name has clap's `get_` prefix.
    #[must_use]
    pub const fn get_without_terminal_detection(&self) -> Option<&'static str> {
        self.without_terminal_detection
    }

    /// Reports whether crumbkit detects terminals for this guide's tool,
    /// which is true unless the tool called
    /// [`Guide::without_terminal_detection`].
    pub(crate) const fn terminal_detection(&self) -> bool {
        self.without_terminal_detection.is_none()
    }

    /// Reports whether a stream is a terminal for this guide's tool: false
    /// with terminal detection off, without calling `terminal`, and
    /// otherwise what `terminal` reports.
    pub(crate) fn detects(&self, terminal: impl FnOnce() -> bool) -> bool {
        self.terminal_detection() && terminal()
    }
}
