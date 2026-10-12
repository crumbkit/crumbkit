//! The guide's terminal detection setting, which decides whether crumbkit
//! asks whether a stream is a terminal (ADR-0049).

use super::Guide;

#[cfg(test)]
mod tests;

impl Guide {
    /// Turns terminal detection off, so crumbkit never asks whether a
    /// stream is a terminal and runs no agent detection for a question.
    ///
    /// Terminal detection changes only help colour, progress and questions,
    /// and never a result, a failure or a note (ADR-0049). With it off:
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
    ///   fail with `crumbkit::confirmation::needs_yes` as they do with
    ///   `--no-input`.
    ///
    /// [`Output::set_command`](crate::Output::set_command) takes the setting
    /// from the [`Parsed`](crate::Parsed) that `parse` returns, or that
    /// [`Parsed::guided`](crate::Parsed::guided) builds for a tool that
    /// parses with clap alone.
    ///
    /// ```
    /// use crumbkit::help::Guide;
    ///
    /// const GUIDE: Guide = Guide::new().without_terminal_detection();
    /// assert!(!GUIDE.get_terminal_detection());
    /// ```
    #[must_use]
    pub const fn without_terminal_detection(mut self) -> Self {
        self.terminal_detection = false;
        self
    }

    /// Reports whether crumbkit detects terminals for this guide's tool,
    /// which is true unless the tool called
    /// [`Guide::without_terminal_detection`].
    #[must_use]
    pub const fn get_terminal_detection(&self) -> bool {
        self.terminal_detection
    }

    /// Reports whether a stream is a terminal for this guide's tool: false
    /// with terminal detection off, without calling `terminal`, and
    /// otherwise what `terminal` reports.
    pub(crate) fn detects(&self, terminal: impl FnOnce() -> bool) -> bool {
        self.terminal_detection && terminal()
    }
}
