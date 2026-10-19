//! The failures of a child process, each with one of crumbkit's fixed
//! codes.

use std::fmt;
use std::io;

use crumbkit::Failure;

use crate::codes;

/// A failure of a child process. Each variant holds the name of the program.
#[derive(Debug, Failure)]
pub(crate) enum ChildError {
    /// The caller passed `--json`, and the child's output is text that the
    /// terminal shows. It refuses the command line with crumbkit's usage
    /// code, as `--fields` is refused.
    #[failure(
        code = crumbkit::codes::CLI_USAGE,
        hint = "run the command without --json to read the output of `{program}`",
        field(argument = "--json"),
    )]
    JsonRefused {
        /// The program's name.
        program: String,
    },
    /// The program does not exist.
    #[failure(code = codes::NOT_FOUND, hint = "install `{program}`, or put it on PATH")]
    NotFound {
        /// The program's name.
        program: String,
    },
    /// The operating system refused to start the program or to wait for it.
    #[failure(
        code = codes::NOT_STARTED,
        hint = "check that `{program}` is an executable file",
    )]
    NotStarted {
        /// The program's name.
        program: String,
        /// Why the program did not start.
        error: io::Error,
    },
}

impl ChildError {
    /// Returns the failure of `program`, which did not start or could not
    /// be waited for because of `error`.
    pub(crate) fn start(program: String, error: io::Error) -> Self {
        if error.kind() == io::ErrorKind::NotFound {
            Self::NotFound { program }
        } else {
            Self::NotStarted { program, error }
        }
    }
}

impl fmt::Display for ChildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::JsonRefused { program } => write!(
                f,
                "--json is not available, because this command hands the terminal to `{program}`"
            ),
            Self::NotFound { program } => write!(f, "cannot run `{program}`: no such program"),
            Self::NotStarted { program, .. } => write!(f, "cannot run `{program}`"),
        }
    }
}

impl std::error::Error for ChildError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NotStarted { error, .. } => Some(error),
            Self::JsonRefused { .. } | Self::NotFound { .. } => None,
        }
    }
}
