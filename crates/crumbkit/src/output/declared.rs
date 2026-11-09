//! The record of what a tool declares about what a run prints on standard
//! output: the output budget (ADR-0046), the layout of its JSON (ADR-0052)
//! and whether crumbkit detects terminals (ADR-0049).

use crate::JsonLayout;

#[cfg(test)]
mod tests;

/// What a tool declares about what a run prints: the output budget, the
/// layout of JSON documents and whether crumbkit detects terminals.
///
/// [`Tool::output`](crate::Tool::output) takes it. Its path is
/// `crumbkit::tool::Output`, because [`crumbkit::Output`](crate::Output)
/// is the handle of one run.
///
/// ```
/// use crumbkit::Tool;
/// use crumbkit::tool;
///
/// const TOOL: Tool =
///     Tool::new().output(tool::Output::new().budget(4096, "A pager reads the output."));
/// assert_eq!(TOOL.get_output().get_budget().map(|(bytes, _)| bytes), Some(4096));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Output {
    /// The most bytes that one run prints on standard output, with the
    /// reason the tool gives for it, or `None` for crumbkit's default.
    budget: Option<(usize, &'static str)>,
    /// The layout of a whole JSON document (ADR-0052).
    json_layout: JsonLayout,
    /// The reason the tool gives for turning terminal detection off, or
    /// `None` when crumbkit asks whether a stream is a terminal (ADR-0049).
    without_terminal_detection: Option<&'static str>,
}

impl Output {
    /// Returns the record of crumbkit's default budget, JSON on one line
    /// and terminal detection on.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            budget: None,
            json_layout: JsonLayout::OneLine,
            without_terminal_detection: None,
        }
    }

    /// Sets the most bytes that one run prints on standard output, and the
    /// reason the tool needs that budget, which crumbkit-test's opt-out
    /// report shows beside crumbkit's default of 1 MiB (ADR-0046).
    ///
    /// A budget can be any number of bytes, as small as the one that
    /// `<TOOL>_OUTPUT_BUDGET` lowers a run to. A result that the budget
    /// refuses prints its over-budget failure whatever the failure's size,
    /// and a document marked with `Document::always_print_first` prints its
    /// first answer. With a budget of 0 bytes, every result is refused or
    /// cut, except the first answer of such a document (ADR-0081, ADR-0084).
    #[must_use]
    pub const fn budget(mut self, bytes: usize, reason: &'static str) -> Self {
        self.budget = Some((bytes, reason));
        self
    }

    /// Returns the budget and the reason that
    /// [`tool::Output::budget`](crate::tool::Output::budget) set, or `None` for
    /// crumbkit's default. The name has clap's `get_` prefix.
    #[must_use]
    pub const fn get_budget(&self) -> Option<(usize, &'static str)> {
        self.budget
    }

    /// Sets the layout of every whole JSON document that the tool prints.
    /// [`JsonLayout::Pretty`] carries the reason the tool needs it, which
    /// crumbkit-test's opt-out report shows with its cost (ADR-0052).
    ///
    /// Without it, a JSON document prints on one line. With
    /// [`JsonLayout::Pretty`], the envelope, a document from
    /// [`Envelope::Without`](crate::Envelope::Without) and the failure
    /// document print indented by two spaces, on standard output whether or
    /// not it is a terminal. A stream keeps one document per line, including
    /// a failure inside it, because JSON Lines needs it. The output budget
    /// counts the indented bytes, so an indented result fits fewer rows.
    ///
    /// ```
    /// use crumbkit::JsonLayout;
    /// use crumbkit::Tool;
    /// use crumbkit::tool;
    ///
    /// const PRETTY: JsonLayout = JsonLayout::Pretty {
    ///     reason: "Our scripts read indented JSON.",
    /// };
    /// const TOOL: Tool = Tool::new().output(tool::Output::new().json_layout(PRETTY));
    /// assert_eq!(TOOL.get_output().get_json_layout(), PRETTY);
    /// assert_eq!(Tool::new().get_output().get_json_layout(), JsonLayout::OneLine);
    /// ```
    #[must_use]
    pub const fn json_layout(mut self, layout: JsonLayout) -> Self {
        self.json_layout = layout;
        self
    }

    /// Returns the layout that
    /// [`tool::Output::json_layout`](crate::tool::Output::json_layout) set, or
    /// [`JsonLayout::OneLine`] when the tool record sets none. The name has
    /// clap's `get_` prefix.
    #[must_use]
    pub const fn get_json_layout(&self) -> JsonLayout {
        self.json_layout
    }

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
    /// - The tool record's [picker](crate::Tool::picker) never runs, and `parse`
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
    /// from the tool record, and so does the one that
    /// [`Output::new`](crate::Output::new) builds for a tool that parses
    /// with clap alone.
    ///
    /// ```
    /// use crumbkit::Tool;
    /// use crumbkit::tool;
    ///
    /// const TOOL: Tool = Tool::new()
    ///     .output(tool::Output::new().without_terminal_detection("Our CI runs in a pseudo-terminal."));
    /// assert_eq!(
    ///     TOOL.get_output().get_without_terminal_detection(),
    ///     Some("Our CI runs in a pseudo-terminal.")
    /// );
    /// ```
    #[must_use]
    pub const fn without_terminal_detection(mut self, reason: &'static str) -> Self {
        self.without_terminal_detection = Some(reason);
        self
    }

    /// Returns the reason that
    /// [`tool::Output::without_terminal_detection`](crate::tool::Output::without_terminal_detection)
    /// set, or `None` when crumbkit detects terminals for this tool. The name
    /// has clap's `get_` prefix.
    #[must_use]
    pub const fn get_without_terminal_detection(&self) -> Option<&'static str> {
        self.without_terminal_detection
    }

    /// Reports whether crumbkit detects terminals for this tool, which is true
    /// unless the tool called
    /// [`tool::Output::without_terminal_detection`](crate::tool::Output::without_terminal_detection).
    pub(crate) const fn terminal_detection(&self) -> bool {
        self.without_terminal_detection.is_none()
    }

    /// Reports whether a stream is a terminal for this tool: false
    /// with terminal detection off, without calling `terminal`, and
    /// otherwise what `terminal` reports.
    pub(crate) fn detects(&self, terminal: impl FnOnce() -> bool) -> bool {
        self.terminal_detection() && terminal()
    }
}

impl Default for Output {
    /// Returns [`Output::new`]. `JsonLayout` has no `Default`, so the record
    /// cannot derive it.
    fn default() -> Self {
        Self::new()
    }
}
