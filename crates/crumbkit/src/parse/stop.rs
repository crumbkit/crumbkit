//! `Stop`, which `parse` returns when it ended the run itself.

use std::process::ExitCode;

use clap::Command;

use crate::Output;
use crate::help::Guide;
use crate::view::{Selection, Table, View};

/// What [`parse`](crate::parse) returns after it printed help, the version,
/// a topic, a refusal or a picker's failure, so the run ends without a
/// parsed command.
/// [`try_parse`](crate::try_parse) returns it inside
/// [`Unparsed::Stopped`](crate::Unparsed::Stopped) after help or the
/// version, and [`Refused::print`](crate::Refused::print) returns it after
/// the refusal prints.
///
/// A tool's `main` returns its exit code, as in `Err(stop) => return
/// stop.code()`. A tool that does more before it exits reads
/// [`Stop::output`], whose [`Output::is_json`] says whether the caller asked
/// for JSON, so the tool knows the format of what it prints next.
///
/// The output sits in a `Box`, so a `Result` that holds a `Stop` stays small
/// on the path that parses.
#[derive(Debug)]
pub struct Stop {
    /// The exit code for `main` to return, as a number.
    code: u8,
    /// What printed before the run ended.
    printed: Printed,
    /// The output flags as far as the walk of the typed words found them,
    /// with the command tree that `parse` built.
    output: Box<Output>,
}

/// What printed before [`parse`](crate::parse) ended a run, which
/// [`Stop::printed`] returns.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Printed {
    /// A help page printed on standard output, for `-h`, `--help` or the
    /// word `help`. The run observer's
    /// [`helped`](crate::events::Observer::helped) event carries whether it
    /// was the long page.
    Help,
    /// The version printed on standard output, for `--version`.
    Version,
    /// A help topic printed on standard output, for the word `help` at the
    /// guide's root followed by the topic's name.
    Topic,
    /// A refused command line printed on standard error, or on standard
    /// output as a JSON document when the caller asked for JSON.
    Refusal,
    /// The failure of a picker's source printed on standard error, or on
    /// standard output as a JSON document when the caller asked for JSON.
    Failure,
}

impl Stop {
    /// Returns the stop of a run that ends with `code` after `printed`
    /// printed, in which the walk found `--json` when `json` is true, after
    /// parsing against `command` with `guide`.
    pub(crate) fn new(
        printed: Printed,
        code: u8,
        json: bool,
        command: Command,
        guide: &Guide,
    ) -> Self {
        Self::with_output(printed, code, stopped(json, command, guide))
    }

    /// Returns the stop of a run that ends with `code` after `printed`
    /// printed, and whose output flags are `output`.
    pub(crate) fn with_output(printed: Printed, code: u8, output: Output) -> Self {
        Self {
            code,
            printed,
            output: Box::new(output),
        }
    }

    /// Returns the exit code for `main` to return: 0 after help or the
    /// version, 2 after a refusal unless the guide lists another code for
    /// it, and 1 when the page could not be written.
    #[must_use]
    pub fn code(&self) -> ExitCode {
        ExitCode::from(self.code)
    }

    /// Returns the exit code that [`Stop::code`] returns, as a number, such
    /// as the code that the guide lists for a refused command line. A tool
    /// that records the exit of a stopped run, or passes it on, reads it
    /// here, because `ExitCode` cannot be read back as a number.
    #[must_use]
    pub const fn number(&self) -> u8 {
        self.code
    }

    /// Returns what printed before the run ended. A tool that records the
    /// outcome of a stopped run in a history of its own reads it here,
    /// because [`Stop::number`] is 0 after help, the version and a topic
    /// alike. A page that could not be written still reports what `parse`
    /// tried to print, with the number 1.
    #[must_use]
    pub const fn printed(&self) -> Printed {
        self.printed
    }

    /// Returns the output flags of the run that `parse` ended. Its
    /// [`Output::is_json`] and [`Output::progress_allowed`] answer as they
    /// do after parsing. Its `--long`, `--full` and `--fields` are always
    /// off, because they shape a result that a stopped run does not have.
    #[must_use]
    pub fn output(&self) -> &Output {
        &self.output
    }
}

/// Returns the output of a run that `parse` ended after parsing against
/// `command` with `guide`, in which the walk found `--json` when `json` is
/// true. Its `--long`, `--full` and `--fields` are off.
pub(crate) fn stopped(json: bool, command: Command, guide: &Guide) -> Output {
    let view = if json {
        View::Json(Selection::Columns)
    } else {
        View::Text(Table::Compact)
    };
    Output::stopped(view, command, guide)
}

impl From<Stop> for ExitCode {
    fn from(stop: Stop) -> Self {
        Self::from(stop.code)
    }
}
