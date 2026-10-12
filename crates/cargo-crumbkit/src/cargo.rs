//! Builds the binary that `weigh` measures and `docs` describes with cargo, and finds its
//! executable and its package in cargo's JSON build messages.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

use crate::error::ToolError;

#[cfg(test)]
mod tests;

/// The binary that `cargo build` built: its executable and the directory of
/// its package's manifest.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Built {
    /// The path of the executable.
    pub(crate) executable: PathBuf,
    /// The directory that holds the package's `Cargo.toml`.
    pub(crate) package: PathBuf,
}

/// Returns the cargo program to run: the path in `CARGO`, which cargo sets
/// when it runs a subcommand such as `cargo crumbkit`, or `cargo` from
/// `PATH` when the variable is unset.
pub(crate) fn program() -> OsString {
    #[expect(
        clippy::disallowed_methods,
        reason = "this module owns weigh's environment read, which finds the cargo that ran it"
    )]
    std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"))
}

/// Builds the binary target `bin` of the workspace in the current
/// directory with `cargo`, and returns its executable and package.
///
/// cargo runs as `cargo build --workspace --bin <bin> --message-format
/// json`, with the environment of `weigh`. `weigh` captures cargo's
/// standard output, which holds the JSON messages, and cargo's standard
/// error goes to `weigh`'s own, so a person sees cargo's progress lines and
/// compiler errors as with any cargo subcommand. `--workspace` makes cargo
/// look for `bin` in every package of the workspace, so `weigh` finds it
/// from any package's directory.
///
/// # Errors
///
/// Returns [`ToolError::CargoUnavailable`] when `cargo` cannot start,
/// [`ToolError::BuildFailed`] when it exits with a failure, and
/// [`ToolError::NoBinary`] when its messages name no executable for `bin`.
pub(crate) fn build(cargo: &OsStr, bin: &str) -> Result<Built, ToolError> {
    #[expect(
        clippy::disallowed_methods,
        reason = "this module owns weigh's cargo process"
    )]
    let output = Command::new(cargo)
        .args([
            "build",
            "--workspace",
            "--bin",
            bin,
            "--message-format",
            "json",
        ])
        .stdin(Stdio::null())
        .stderr(Stdio::inherit())
        .output()
        .map_err(|error| ToolError::CargoUnavailable {
            cargo: cargo.to_string_lossy().into_owned(),
            error,
        })?;
    if !output.status.success() {
        return Err(ToolError::BuildFailed {
            bin: bin.to_owned(),
            status: output.status.to_string(),
        });
    }
    built(&output.stdout, bin).ok_or_else(|| ToolError::NoBinary {
        bin: bin.to_owned(),
    })
}

/// Returns the executable and package of the binary target `bin` from
/// `messages`, cargo's JSON build messages with one message per line, or
/// `None` when no `compiler-artifact` message holds an executable for it.
///
/// A line that is not JSON, or a message of another kind, is skipped,
/// because cargo also prints messages that `weigh` does not need, such as
/// `build-finished`.
fn built(messages: &[u8], bin: &str) -> Option<Built> {
    messages
        .split(|&byte| byte == b'\n')
        .filter_map(|line| serde_json::from_slice::<Value>(line).ok())
        .find_map(|message| artifact(&message, bin))
}

/// Returns the executable and package that one cargo message names, when it
/// is the `compiler-artifact` message of the binary target `bin`.
fn artifact(message: &Value, bin: &str) -> Option<Built> {
    if message.get("reason")?.as_str()? != "compiler-artifact" {
        return None;
    }
    let target = message.get("target")?;
    let is_bin = target
        .get("kind")?
        .as_array()?
        .iter()
        .any(|kind| kind.as_str() == Some("bin"));
    if !is_bin || target.get("name")?.as_str()? != bin {
        return None;
    }
    let executable = PathBuf::from(message.get("executable")?.as_str()?);
    let manifest = Path::new(message.get("manifest_path")?.as_str()?);
    Some(Built {
        executable,
        package: manifest.parent()?.to_path_buf(),
    })
}
