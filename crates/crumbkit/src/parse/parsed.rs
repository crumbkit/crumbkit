//! `Parsed`, the command tree that a tool parsed against with the words that
//! the caller typed.

use std::ffi::OsString;

use clap::Command;

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
/// words, so `confirm` has no command line to repeat.
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
}

impl Parsed {
    /// Returns the command tree, the words after the program name, which
    /// hold any values that a picker added, and whether the caller passed
    /// `--no-input`.
    pub(crate) const fn new(command: Command, words: Vec<OsString>, no_input: bool) -> Self {
        Self {
            command,
            words: Some(words),
            no_input,
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

/// Returns a `Parsed` without words, for a tool that parsed with clap alone.
impl From<Command> for Parsed {
    fn from(command: Command) -> Self {
        Self {
            command,
            words: None,
            no_input: false,
        }
    }
}
