//! `Parsed`, the command tree that a tool parsed against with the words that
//! the caller typed.

use std::ffi::OsString;

use clap::Command;

use crate::JsonLayout;
use crate::form::FailureForm;
use crate::help::{Exit, Guide, Rename};

/// The command tree that [`parse`](crate::parse) parsed against, with the
/// words that the caller typed after the program name and the values that
/// the guide's picker added to them.
///
/// A tool's `main` passes it to
/// [`Output::set_command`](crate::Output::set_command), which keeps both.
/// `finish` renders calls against the tree, and
/// [`Output::confirm`](crate::Output::confirm) repeats the words with
/// `--yes` as the recovery of a run that needs it, and fails without asking
/// when the caller passed `--no-input`. A tool that parses with
/// clap alone passes its `Command`, which converts into a `Parsed` without
/// words, so `confirm` has no command line to repeat, or builds it with
/// [`Parsed::guided`] when its guide sets a failure form, an output budget,
/// a JSON layout or exit codes.
#[derive(Clone, Debug)]
pub struct Parsed {
    /// The command tree, built as parsing left it.
    pub(crate) command: Command,
    /// The words after the program name, or `None` for a `Parsed` made from
    /// a bare `Command`.
    pub(crate) words: Option<Vec<OsString>>,
    /// Whether the caller passed `--no-input`, so no person can answer a
    /// question.
    pub(crate) no_input: bool,
    /// The failure form of the tool's guide, or `None` for crumbkit's form.
    pub(crate) failure_form: Option<&'static dyn FailureForm>,
    /// Whether the guide turned terminal detection off (ADR-0049).
    pub(crate) no_terminal: bool,
    /// The output budget of the tool's guide, or `None` for crumbkit's
    /// default.
    pub(crate) output_budget: Option<usize>,
    /// The layout of a whole JSON document that the tool's guide sets, or
    /// `None` for one line (ADR-0052).
    pub(crate) json_layout: Option<JsonLayout>,
    /// The exit codes that the tool's guide declares, whose failure codes
    /// choose a failed run's exit code (ADR-0037).
    pub(crate) exits: &'static [Exit],
    /// The tool's own codes for crumbkit's failures (ADR-0056).
    pub(crate) renames: &'static [Rename],
}

impl Parsed {
    /// Returns the command tree, the words after the program name, which
    /// hold any values that a picker added, whether the caller passed
    /// `--no-input`, and the failure form, terminal detection, output budget
    /// and JSON layout of `guide`, and the exit codes and renames that it
    /// declares.
    pub(crate) const fn new(
        command: Command,
        words: Vec<OsString>,
        no_input: bool,
        guide: &Guide,
    ) -> Self {
        Self {
            command,
            words: Some(words),
            no_input,
            failure_form: guide.get_failure_form(),
            no_terminal: !guide.terminal_detection(),
            output_budget: budget_of(guide),
            json_layout: guide.layout(),
            exits: guide.get_exits(),
            renames: guide.get_codes(),
        }
    }

    /// Returns a `Parsed` without words for a tool that parsed `command`
    /// with clap alone, which keeps the failure form, the output budget, the
    /// JSON layout, the exit codes and the renames of `guide`, so `finish`
    /// prints failures in the form (ADR-0045) with the tool's codes
    /// (ADR-0056), results within the budget (ADR-0046) and documents in the
    /// layout (ADR-0052), and exits with the code that the guide declares
    /// for a failure (ADR-0037), and whether `guide` turns
    /// terminal detection off (ADR-0049). A tool whose guide sets none of
    /// them passes its `Command` to `Output::set_command` directly.
    #[must_use]
    pub const fn guided(command: Command, guide: &Guide) -> Self {
        Self {
            command,
            words: None,
            no_input: false,
            failure_form: guide.get_failure_form(),
            no_terminal: !guide.terminal_detection(),
            output_budget: budget_of(guide),
            json_layout: guide.layout(),
            exits: guide.get_exits(),
            renames: guide.get_codes(),
        }
    }

    /// Returns the command tree.
    #[must_use]
    pub const fn command(&self) -> &Command {
        &self.command
    }

    /// Returns the command tree and drops the words.
    #[must_use]
    pub fn into_command(self) -> Command {
        self.command
    }
}

/// Returns the bytes of the output budget that `guide` sets, or `None`.
const fn budget_of(guide: &Guide) -> Option<usize> {
    match guide.get_output_budget() {
        Some((bytes, _)) => Some(bytes),
        None => None,
    }
}

/// Returns a `Parsed` without words, for a tool that parsed with clap alone.
impl From<Command> for Parsed {
    fn from(command: Command) -> Self {
        Self {
            command,
            words: None,
            no_input: false,
            failure_form: None,
            no_terminal: false,
            output_budget: None,
            json_layout: None,
            exits: &[],
            renames: &[],
        }
    }
}
