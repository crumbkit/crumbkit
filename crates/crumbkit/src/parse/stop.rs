//! `Stop`, which `parse` returns when it ended the run itself.

use std::process::ExitCode;

use clap::Command;

use crate::Output;

/// What [`parse`](crate::parse) returns after it printed help, the version
/// or a refusal, so the run ends without a parsed command.
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
    /// The exit code for `main` to return.
    code: ExitCode,
    /// The output flags as far as the walk of the typed words found them,
    /// with the command tree that `parse` built.
    output: Box<Output>,
}

impl Stop {
    /// Returns the stop of a run that ends with `code`, in which the walk
    /// found `--json` when `json` is true, after parsing against `command`.
    pub(crate) fn new(code: ExitCode, json: bool, command: Command) -> Self {
        Self {
            code,
            output: Box::new(Output::stopped(json, command)),
        }
    }

    /// Returns the exit code for `main` to return: 0 after help or the
    /// version, 2 after a refusal, and 1 when the page could not be written.
    #[must_use]
    pub const fn code(&self) -> ExitCode {
        self.code
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

impl From<Stop> for ExitCode {
    fn from(stop: Stop) -> Self {
        stop.code
    }
}
