//! The failures of `cargo crumbkit weigh`.

use std::fmt;
use std::io;
use std::path::PathBuf;

use crumbkit::{Code, Failure, Recovery};

#[cfg(test)]
mod tests;

/// The code of a failure whose cargo cannot start.
const CARGO_UNAVAILABLE: Code = Code::new("crumbkit", "weigh", "cargo_unavailable");

/// The code of a failure whose `cargo build` exits with a failure.
const BUILD_FAILED: Code = Code::new("crumbkit", "weigh", "build_failed");

/// The code of a failure whose build names no executable for the binary.
const NO_BINARY: Code = Code::new("crumbkit", "weigh", "no_binary");

/// The code of a failure whose temporary XDG directories cannot be created.
const NO_TEMP_DIR: Code = Code::new("crumbkit", "weigh", "no_temp_dir");

/// The code of a failure whose tool binary cannot start.
const TOOL_UNAVAILABLE: Code = Code::new("crumbkit", "weigh", "tool_unavailable");

/// The code of a failure whose tool prints no `commands list` document.
const NO_COMMANDS_LIST: Code = Code::new("crumbkit", "weigh", "no_commands_list");

/// The code of a failure whose tool fails to print a help page.
const HELP_FAILED: Code = Code::new("crumbkit", "weigh", "help_failed");

/// The code of a failure whose golden file exists but cannot be read.
const GOLDEN_UNREADABLE: Code = Code::new("crumbkit", "weigh", "golden_unreadable");

/// The code of a failure whose tool prints no `self figures list` document.
const NO_USAGE_FIGURES: Code = Code::new("crumbkit", "weigh", "no_usage_figures");

/// The code of a failure whose usage log cannot be found, because the XDG
/// state directory cannot be found.
const NO_STATE_DIR: Code = Code::new("crumbkit", "weigh", "no_state_dir");

/// A failure of `cargo crumbkit weigh`.
#[derive(Debug)]
pub(crate) enum WeighError {
    /// cargo cannot start.
    CargoUnavailable {
        /// The cargo program that `weigh` ran.
        cargo: String,
        /// Why it cannot start.
        error: io::Error,
    },
    /// `cargo build` exits with a failure.
    BuildFailed {
        /// The name of the binary that `weigh` builds.
        bin: String,
        /// How cargo exited, such as `exit status: 101`.
        status: String,
    },
    /// cargo's build messages name no executable for the binary.
    NoBinary {
        /// The name of the binary that `weigh` builds.
        bin: String,
    },
    /// The temporary XDG directories cannot be created.
    NoTempDir {
        /// Why they cannot be created.
        error: io::Error,
    },
    /// The tool's binary cannot start.
    ToolUnavailable {
        /// The path of the binary.
        binary: PathBuf,
        /// Why it cannot start.
        error: io::Error,
    },
    /// The tool's `commands list --all --json --full` prints no
    /// `crumbkit/commands-list/v1` document.
    NoCommandsList {
        /// The command line that `weigh` ran, such as
        /// `shelf commands list --all --json --full`.
        line: String,
    },
    /// The tool's run for a help page exits with a failure.
    HelpFailed {
        /// The command line that `weigh` ran, such as `shelf books -h`.
        line: String,
        /// How the tool exited, such as `exit status: 2`.
        status: String,
    },
    /// A golden file exists but cannot be read.
    GoldenUnreadable {
        /// The path of the golden file.
        path: PathBuf,
        /// Why it cannot be read.
        error: io::Error,
    },
    /// The tool's `self figures list --all --json --log <FILE>` prints no
    /// `crumbkit/self-figures-list/v1` document.
    NoUsageFigures {
        /// The command line that `weigh` ran, such as
        /// `shelf self figures list --all --json --log usage.jsonl`.
        line: String,
    },
    /// `--usage` names no log, and neither `XDG_STATE_HOME` nor `HOME`
    /// gives the state directory that holds the tool's own log. Windows
    /// always has `%LOCALAPPDATA%`, so this happens only on Unix.
    NoStateDir,
}

impl WeighError {
    /// Every code that a `WeighError` returns. Only the tests read the list,
    /// which they pass to crumbkit-test's codes check.
    #[cfg(test)]
    pub(crate) const CODES: &'static [&'static Code] = &[
        &CARGO_UNAVAILABLE,
        &BUILD_FAILED,
        &NO_BINARY,
        &NO_TEMP_DIR,
        &TOOL_UNAVAILABLE,
        &NO_COMMANDS_LIST,
        &HELP_FAILED,
        &GOLDEN_UNREADABLE,
        &NO_USAGE_FIGURES,
        &NO_STATE_DIR,
    ];
}

/// Prints what failed, such as ``` `cargo build --workspace --bin shelf`
/// exited with exit status: 101 ```.
impl fmt::Display for WeighError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CargoUnavailable { cargo, .. } => write!(f, "cargo cannot start as `{cargo}`"),
            Self::BuildFailed { bin, status } => {
                write!(
                    f,
                    "`cargo build --workspace --bin {bin}` exited with {status}"
                )
            }
            Self::NoBinary { bin } => write!(f, "cargo built no executable for the binary {bin}"),
            Self::NoTempDir { .. } => {
                f.write_str("the temporary XDG directories for the tool's runs cannot be created")
            }
            Self::ToolUnavailable { binary, .. } => {
                write!(f, "the binary {} cannot start", binary.display())
            }
            Self::NoCommandsList { line } => write!(
                f,
                "`{line}` prints no crumbkit/commands-list/v1 document, so the tool's commands cannot be read"
            ),
            Self::HelpFailed { line, status } => write!(f, "`{line}` exited with {status}"),
            Self::GoldenUnreadable { path, .. } => {
                write!(f, "the golden file {} cannot be read", path.display())
            }
            Self::NoUsageFigures { line } => write!(
                f,
                "`{line}` prints no crumbkit/self-figures-list/v1 document, so the usage figures cannot be read"
            ),
            Self::NoStateDir => f.write_str(
                "the usage log cannot be found, because the XDG state directory cannot be found",
            ),
        }
    }
}

/// A failure caused by an I/O error has that error as its source, which
/// prints as the failure's cause.
impl std::error::Error for WeighError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CargoUnavailable { error, .. }
            | Self::NoTempDir { error }
            | Self::ToolUnavailable { error, .. }
            | Self::GoldenUnreadable { error, .. } => Some(error),
            Self::BuildFailed { .. }
            | Self::NoBinary { .. }
            | Self::NoCommandsList { .. }
            | Self::HelpFailed { .. }
            | Self::NoUsageFigures { .. }
            | Self::NoStateDir => None,
        }
    }
}

/// Each failure's next step is advice, because each one needs a change
/// outside cargo crumbkit: to cargo, to the workspace or to the tool.
impl Failure for WeighError {
    fn code(&self) -> &'static Code {
        match self {
            Self::CargoUnavailable { .. } => &CARGO_UNAVAILABLE,
            Self::BuildFailed { .. } => &BUILD_FAILED,
            Self::NoBinary { .. } => &NO_BINARY,
            Self::NoTempDir { .. } => &NO_TEMP_DIR,
            Self::ToolUnavailable { .. } => &TOOL_UNAVAILABLE,
            Self::NoCommandsList { .. } => &NO_COMMANDS_LIST,
            Self::HelpFailed { .. } => &HELP_FAILED,
            Self::GoldenUnreadable { .. } => &GOLDEN_UNREADABLE,
            Self::NoUsageFigures { .. } => &NO_USAGE_FIGURES,
            Self::NoStateDir => &NO_STATE_DIR,
        }
    }

    fn recovery(&self) -> Recovery {
        let hint = match self {
            Self::CargoUnavailable { .. } => {
                "run cargo crumbkit through cargo, which sets CARGO to its own path".to_owned()
            }
            Self::BuildFailed { bin, .. } => {
                format!("run `cargo build --workspace --bin {bin}` to see cargo's errors")
            }
            Self::NoBinary { bin } => format!(
                "run `cargo build --workspace --bin {bin} --message-format json` to see the artifacts that cargo built"
            ),
            Self::NoTempDir { .. } => {
                "make the system's temporary directory writable, or set TMPDIR to one that is"
                    .to_owned()
            }
            Self::ToolUnavailable { .. } => {
                "check that the binary runs on this machine".to_owned()
            }
            Self::NoCommandsList { .. } => "add a `commands list` command at the tool's root that takes crumbkit::commands::CommandsList, as the crumbkit::commands module shows".to_owned(),
            Self::HelpFailed { line, .. } => format!("run `{line}` to see the failure"),
            Self::GoldenUnreadable { .. } => {
                "make the golden directory readable, or pass --goldens another directory"
                    .to_owned()
            }
            Self::NoUsageFigures { line } => format!(
                "run `{line}` to see the failure; a tool needs a `self figures list` command that takes crumbkit::usage::SelfFigures, as shelf's shows"
            ),
            Self::NoStateDir => "set HOME or XDG_STATE_HOME, or pass --log the usage log".to_owned(),
        };
        Recovery::Hint(hint)
    }
}
