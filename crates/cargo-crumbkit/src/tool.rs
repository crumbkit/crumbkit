//! Runs the tool's built binary, as an agent would, to read its command tree
//! and to measure its help pages.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde::Deserialize;
use tempfile::TempDir;

use crate::error::WeighError;

#[cfg(test)]
mod tests;

/// The XDG base directory variables that each run sets, with the name of the
/// directory in the temporary directory that each one points at.
const XDG: [(&str, &str); 4] = [
    ("XDG_CONFIG_HOME", "config"),
    ("XDG_DATA_HOME", "data"),
    ("XDG_STATE_HOME", "state"),
    ("XDG_CACHE_HOME", "cache"),
];

/// The id of the document that `commands list --json` prints.
const COMMANDS: &str = "crumbkit/commands-list/v1";

/// The words after the binary that list every visible command with its
/// examples as JSON.
const COMMANDS_LIST: [&str; 5] = ["commands", "list", "--all", "--json", "--full"];

/// One visible command of the tool, as `commands list` prints it.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Listed {
    /// The command line that runs the command, such as `shelf books list`.
    pub(crate) path: String,
    /// The command lines of the command's examples, such as
    /// `shelf books list --all`.
    pub(crate) examples: Vec<String>,
}

/// The `crumbkit/commands-list/v1` document, with only the fields that `weigh`
/// reads. serde ignores the other fields.
#[derive(Deserialize)]
struct CommandsDocument {
    /// The document's id.
    schema: String,
    /// One row for each visible command, in depth-first order.
    data: Vec<CommandRow>,
}

/// One row of the `crumbkit/commands-list/v1` document.
#[derive(Deserialize)]
struct CommandRow {
    /// The command line that runs the command.
    path: String,
    /// The command's examples.
    examples: Vec<CommandExample>,
}

/// One example of a row of the `crumbkit/commands-list/v1` document.
#[derive(Deserialize)]
struct CommandExample {
    /// The example's command line, with words quoted for a POSIX shell.
    command: String,
}

/// The tool's built binary, which runs with a cleared environment and XDG
/// directories in a temporary directory, so a run never reads the person's
/// aliases or writes to their usage log.
pub(crate) struct Tool {
    /// The path of the binary.
    binary: PathBuf,
    /// The temporary directory that holds the XDG directories, which every
    /// run of one `weigh` shares, and which is deleted when the value drops.
    xdg: TempDir,
}

impl Tool {
    /// Returns the tool whose binary is at `binary`, with fresh XDG
    /// directories in a new temporary directory inside `temp`, which `weigh`
    /// gives as the system's temporary directory.
    ///
    /// # Errors
    ///
    /// Returns [`WeighError::NoTempDir`] when the XDG directories cannot be
    /// created.
    pub(crate) fn new(binary: &Path, temp: &Path) -> Result<Self, WeighError> {
        let xdg = TempDir::new_in(temp).map_err(|error| WeighError::NoTempDir { error })?;
        for (_, name) in XDG {
            #[expect(
                clippy::disallowed_methods,
                reason = "the tool owns its temporary XDG directories"
            )]
            fs::create_dir_all(xdg.path().join(name))
                .map_err(|error| WeighError::NoTempDir { error })?;
        }
        Ok(Self {
            binary: binary.to_path_buf(),
            xdg,
        })
    }

    /// Returns every visible command of the tool in depth-first order from
    /// the root, read from `<binary> commands list --all --json --full`.
    ///
    /// # Errors
    ///
    /// Returns [`WeighError::ToolUnavailable`] when the binary cannot start,
    /// and [`WeighError::NoCommandsList`] when the run fails or prints no
    /// `crumbkit/commands-list/v1` document, such as for a tool that does not
    /// mount `commands list`.
    pub(crate) fn commands(&self) -> Result<Vec<Listed>, WeighError> {
        let output = self.run(&COMMANDS_LIST)?;
        let listed = if output.status.success() {
            listed(&output.stdout)
        } else {
            None
        };
        listed.ok_or_else(|| {
            let name = self
                .binary
                .file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
            WeighError::NoCommandsList {
                line: std::iter::once(name.as_str())
                    .chain(COMMANDS_LIST)
                    .collect::<Vec<_>>()
                    .join(" "),
            }
        })
    }

    /// Returns the number of bytes that the tool writes to standard output
    /// for the command at `path`, such as `shelf books list`, followed by
    /// `flag`, `-h` or `--help`.
    ///
    /// # Errors
    ///
    /// Returns [`WeighError::ToolUnavailable`] when the binary cannot start,
    /// and [`WeighError::HelpFailed`] when the run exits with a failure.
    pub(crate) fn help_bytes(&self, path: &str, flag: &str) -> Result<u64, WeighError> {
        // The first word of the path is the tool's name, which the binary
        // stands for. A command's name never holds a space, because help and
        // `commands list` join the names with spaces.
        let mut args: Vec<&str> = path.split(' ').skip(1).collect();
        args.push(flag);
        let output = self.run(&args)?;
        if !output.status.success() {
            return Err(WeighError::HelpFailed {
                line: format!("{path} {flag}"),
                status: output.status.to_string(),
            });
        }
        // A `usize` always fits in a `u64` on the 32-bit and 64-bit
        // platforms that Rust supports, so the conversion never saturates.
        Ok(u64::try_from(output.stdout.len()).unwrap_or(u64::MAX))
    }

    /// Runs the binary with `args`, a cleared environment and the XDG
    /// directories, with no standard input, and returns its output.
    fn run(&self, args: &[&str]) -> Result<Output, WeighError> {
        #[expect(
            clippy::disallowed_methods,
            reason = "this module owns the runs of the tool's binary"
        )]
        let mut command = Command::new(&self.binary);
        command.args(args).env_clear().stdin(Stdio::null());
        for (variable, name) in XDG {
            command.env(variable, self.xdg.path().join(name));
        }
        command
            .output()
            .map_err(|error| WeighError::ToolUnavailable {
                binary: self.binary.clone(),
                error,
            })
    }
}

/// Returns the commands that `stdout`, the output of `commands list --json
/// --full`, lists, or `None` when it is not a `crumbkit/commands-list/v1`
/// document.
fn listed(stdout: &[u8]) -> Option<Vec<Listed>> {
    let document: CommandsDocument = serde_json::from_slice(stdout).ok()?;
    if document.schema != COMMANDS {
        return None;
    }
    let commands = document
        .data
        .into_iter()
        .map(|row| Listed {
            path: row.path,
            examples: row
                .examples
                .into_iter()
                .map(|example| example.command)
                .collect(),
        })
        .collect();
    Some(commands)
}
