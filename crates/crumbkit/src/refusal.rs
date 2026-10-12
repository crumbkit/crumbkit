//! A refusal: a command line that clap refused, or a flag value that crumbkit
//! refused, printed as a failure with one fixed code and exit code 2.

use std::fmt;
use std::io::Write;
use std::process::ExitCode;

use clap::Command;

use crate::output::failure;
use crate::{Call, Code, Failure, Recovery, Sink, Spelling};

/// The code of every refusal, in the tool's spelling, such as
/// `crumbkit::cli::usage` (ADR-0001).
const USAGE: Code = Code::new("crumbkit", "cli", "usage");

/// A refused command line or flag value, as a [`Failure`] with the code
/// `crumbkit::cli::usage`.
#[derive(Debug)]
pub(crate) struct Refusal {
    /// What the caller got wrong, on one line.
    message: String,
    /// What the caller does next.
    recovery: Recovery,
}

impl Refusal {
    /// Returns the refusal of a command line that says `message` and whose
    /// recovery runs the `-h` of the command at `path`, the names of the
    /// subcommands from the root.
    pub(crate) fn command_line(message: String, path: Vec<String>) -> Self {
        Self {
            message,
            recovery: Recovery::Run(Call::help(path)),
        }
    }

    /// Returns the refusal of a flag value that says `message` and whose
    /// recovery is the hint `hint`, such as `use the valid fields: a, b`.
    pub(crate) const fn flag_value(message: String, hint: String) -> Self {
        Self {
            message,
            recovery: Recovery::Hint(hint),
        }
    }

    /// Prints the refusal to `sink` as `finish` prints a failure, as text or
    /// with `json` as one `crumbkit/failure/v1` document, and returns exit
    /// code 2 (ADR-0002). The recovery renders against `command`, so it
    /// prints as having no next step when there is no command.
    pub(crate) fn print<O: Write, E: Write, R>(
        &self,
        sink: &mut Sink<O, E, R>,
        json: bool,
        spelling: Spelling,
        command: Option<&mut Command>,
    ) -> ExitCode {
        failure::print(sink, json, spelling, self, command);
        ExitCode::from(2)
    }
}

/// Prints the message.
impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Refusal {}

impl Failure for Refusal {
    fn code(&self) -> &'static Code {
        &USAGE
    }

    fn recovery(&self) -> Recovery {
        self.recovery.clone()
    }
}
