//! The failures of a child process, each with one of crumbkit's fixed
//! codes.

use std::fmt;
use std::io;

use crumbkit::{Code, Failure, Recovery};

use crate::codes;

/// A failure of a child process. Each variant holds the name of the program.
#[derive(Debug)]
pub(crate) enum ChildError {
    /// The caller passed `--json`, and the child has no JSON form.
    NoJson(String),
    /// The program does not exist.
    NotFound(String),
    /// The operating system refused to start the program or to wait for it.
    NotStarted {
        /// The program's name.
        program: String,
        /// Why the program did not start.
        error: io::Error,
    },
    /// The child's standard output is not the declared document.
    BadJson {
        /// The program's name.
        program: String,
        /// The id of the declared document, such as `vault/log-list/v1`.
        document: String,
        /// Why the output does not parse as the document.
        error: serde_json::Error,
    },
    /// The child exited with a code other than 0 under `--json`.
    Failed {
        /// The program's name.
        program: String,
        /// The child's exit number.
        exit: u8,
    },
}

impl ChildError {
    /// Returns the failure of `program`, which did not start or could not
    /// be waited for because of `error`.
    pub(crate) fn start(program: String, error: io::Error) -> Self {
        if error.kind() == io::ErrorKind::NotFound {
            Self::NotFound(program)
        } else {
            Self::NotStarted { program, error }
        }
    }
}

impl fmt::Display for ChildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoJson(program) => write!(
                f,
                "--json is not available, because this command hands the terminal to `{program}`"
            ),
            Self::NotFound(program) => write!(f, "cannot run `{program}`: no such program"),
            Self::NotStarted { program, .. } => write!(f, "cannot run `{program}`"),
            Self::BadJson {
                program, document, ..
            } => write!(
                f,
                "`{program}` printed output that is not a {document} document"
            ),
            Self::Failed { program, exit } => write!(f, "`{program}` exited with code {exit}"),
        }
    }
}

impl std::error::Error for ChildError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NotStarted { error, .. } => Some(error),
            Self::BadJson { error, .. } => Some(error),
            Self::NoJson(_) | Self::NotFound(_) | Self::Failed { .. } => None,
        }
    }
}

impl Failure for ChildError {
    fn code(&self) -> &'static Code {
        match self {
            Self::NoJson(_) => &codes::NO_JSON,
            Self::NotFound(_) => &codes::NOT_FOUND,
            Self::NotStarted { .. } => &codes::NOT_STARTED,
            Self::BadJson { .. } => &codes::BAD_JSON,
            Self::Failed { .. } => &codes::FAILED,
        }
    }

    fn recovery(&self) -> Recovery {
        match self {
            Self::NoJson(program) => Recovery::Hint(format!(
                "run the command without --json to read the output of `{program}`"
            )),
            Self::NotFound(program) => {
                Recovery::Hint(format!("install `{program}`, or put it on PATH"))
            }
            Self::NotStarted { program, .. } => {
                Recovery::Hint(format!("check that `{program}` is an executable file"))
            }
            Self::BadJson { .. } => Recovery::None {
                reason: "the program's output does not match the document that the tool declares"
                    .to_owned(),
            },
            Self::Failed { .. } => Recovery::None {
                reason: "the program reported its failure on standard error".to_owned(),
            },
        }
    }
}
