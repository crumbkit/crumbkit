//! `Stop`, which `parse` returns when it ended the run itself.

use std::ffi::{OsStr, OsString};
use std::process::ExitCode;

use clap::Command;

use super::walked;
use crate::Output;
use crate::help::Guide;
use crate::view::{Selection, Table, View};

#[cfg(test)]
mod tests;

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
/// for JSON, so the tool knows the format of what it prints next. It reads
/// [`Stop::path`], [`Stop::flags`] and [`Stop::value`] as it reads them on a
/// [`Refused`](crate::Refused), such as a global option that chooses the
/// format of a note it prints after a help page.
///
/// The output and the words sit in a `Box`, so a `Result` that holds a
/// `Stop` stays small on the path that parses.
#[derive(Debug)]
pub struct Stop {
    /// The exit code for `main` to return, as a number.
    code: u8,
    /// What printed before the run ended.
    printed: Printed,
    /// The output flags and the words of the run.
    inner: Box<Inner>,
}

/// What a [`Stop`] holds besides its code and what printed.
#[derive(Debug)]
struct Inner {
    /// The output flags as far as the walk of the typed words found them,
    /// with the command tree that `parse` built.
    output: Output,
    /// The names of the subcommands from the root to the deepest command
    /// that the walk of the words reached.
    path: Vec<String>,
    /// The words after the program name that crumbkit gave clap.
    words: Vec<OsString>,
}

/// What printed before [`parse`](crate::parse) ended a run, which
/// [`Stop::printed`] returns.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Printed {
    /// A help page printed as the result on standard output, for `-h`,
    /// `--help` or the word `help`. The run observer's
    /// [`helped`](crate::events::Observer::helped) event carries whether it
    /// was the long page.
    Help,
    /// The version printed as the result on standard output, for
    /// `--version`.
    Version,
    /// A help topic printed as the result on standard output, for the word
    /// `help` at the guide's root followed by the topic's name.
    Topic,
    /// A refused command line printed as a message on standard error, or as
    /// the result on standard output in a JSON document when the caller
    /// asked for JSON or the tool chose JSON with
    /// [`Refused::as_json`](crate::Refused::as_json).
    Refusal,
    /// The failure of a picker's source printed as a message on standard
    /// error, or as the result on standard output in a JSON document when
    /// the caller asked for JSON.
    Failure,
}

impl Stop {
    /// Returns the stop of a run that ends with `code` after `printed`
    /// printed, in which the walk found `--json` when `json` is true, after
    /// parsing against `command` with `guide`. `path` names the subcommands
    /// that the walk reached, and `words` are the words after the program
    /// name that crumbkit gave clap.
    pub(crate) fn new(
        printed: Printed,
        code: u8,
        json: bool,
        command: Command,
        guide: &Guide,
        path: Vec<String>,
        words: Vec<OsString>,
    ) -> Self {
        let output = stopped(json, command, guide);
        Self::with_output(printed, code, output, path, words)
    }

    /// Returns the stop of a run that ends with `code` after `printed`
    /// printed, whose output flags are `output`, as [`Stop::new`] describes
    /// `path` and `words`.
    pub(crate) fn with_output(
        printed: Printed,
        code: u8,
        output: Output,
        path: Vec<String>,
        words: Vec<OsString>,
    ) -> Self {
        Self {
            code,
            printed,
            inner: Box::new(Inner {
                output,
                path,
                words,
            }),
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
    /// After a refusal whose form the tool chose with
    /// [`Refused::as_json`](crate::Refused::as_json), its `is_json` reports
    /// the form that printed.
    #[must_use]
    pub fn output(&self) -> &Output {
        &self.inner.output
    }

    /// Returns the real names of the subcommands from the root to the
    /// deepest command that the walk of the words reached, such as
    /// `planets` for `tool planets --help`. The path is empty when the walk
    /// reached no subcommand, as for the version and for a topic, which
    /// prints only at the guide's root. A tool that prints a note after a
    /// help page names the command of the page from it.
    #[must_use]
    pub fn path(&self) -> &[String] {
        &self.inner.path
    }

    /// Returns the clap ids of the arguments that the words pass as flags,
    /// in the order typed, including the options that take values. A group
    /// of short flags such as `-qv` gives the id of each flag in it, and a
    /// name that no argument has gives nothing.
    ///
    /// The words are the ones after the program name that crumbkit gave
    /// clap, so `--help` gives clap's help id, `help`, also when crumbkit
    /// turned the word `help` into `--help`. They are read as
    /// [`command_path`](crate::command_path) reads them, so the value of an
    /// option is never read as a flag. After [`Output::new`] refused its
    /// view flags, the words are the ones passed to it.
    #[must_use]
    pub fn flags(&self) -> Vec<String> {
        walked::flags(self.inner.output.command_tree(), &self.inner.words)
    }

    /// Returns the last value that the words give the option whose clap id
    /// is `id`, or `None` when the words give it no value or `id` names no
    /// option. The option is the current command's own argument or a
    /// global argument of a command above it.
    ///
    /// The value is the word after the option's flag, or the text after
    /// the name in the flag's own word, as in `--format=json`, `-fjson` and
    /// `-f=json`. It comes back as one word, so a value that the option
    /// splits at its value delimiter comes back unsplit. The words are read
    /// as [`Stop::flags`] reads them. A tool that prints a note after a help
    /// page reads a global option such as `--format` here to choose how to
    /// print it.
    #[must_use]
    pub fn value(&self, id: &str) -> Option<&OsStr> {
        walked::value(self.inner.output.command_tree(), &self.inner.words, id)
    }
}

/// Returns the output of a run that `parse` ended after parsing against
/// `command` with `guide`, in which the walk found `--json` when `json` is
/// true. Its `--long`, `--full` and `--fields` are off.
pub(crate) fn stopped(json: bool, command: Command, guide: &Guide) -> Output {
    Output::stopped(view(json), command, guide)
}

/// Returns the view of a run that `parse` ended: JSON when `json` is true
/// and text otherwise, with `--long`, `--full` and `--fields` off.
pub(crate) const fn view(json: bool) -> View {
    if json {
        View::Json(Selection::Columns)
    } else {
        View::Text(Table::Compact)
    }
}

impl From<Stop> for ExitCode {
    fn from(stop: Stop) -> Self {
        Self::from(stop.code)
    }
}
