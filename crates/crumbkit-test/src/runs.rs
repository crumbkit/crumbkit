//! The runs of a tool's built binary that the example, command reference,
//! recovery and twin checks make, each with a cleared environment and fresh XDG directories.

use std::path::PathBuf;
use std::process::Output;
use std::time::Duration;

use serde_json::Value;
use snapbox::cmd;

use crate::temp::TempDir;

/// The XDG base directory variables that each run sets, with the name of the
/// fresh directory that each one points at.
const XDG: [(&str, &str); 4] = [
    ("XDG_CONFIG_HOME", "config"),
    ("XDG_DATA_HOME", "data"),
    ("XDG_STATE_HOME", "state"),
    ("XDG_CACHE_HOME", "cache"),
];

/// How long a check lets one run take before it stops the run.
pub(crate) const TIMEOUT: Duration = Duration::from_secs(10);

/// Fresh XDG base directories, which one run or every run of one recovery
/// chain uses, so the runs of a chain see what the earlier runs wrote. The
/// directories are deleted when the value drops.
pub(crate) struct Xdg {
    /// The directory that holds the four XDG directories.
    _dir: TempDir,
    /// Each XDG variable with the path of its directory.
    paths: Vec<(&'static str, PathBuf)>,
}

impl Xdg {
    /// Creates the four XDG directories in a new temporary directory whose
    /// name ends with `label`, or returns the problem when they cannot be
    /// created.
    pub(crate) fn new(label: &str) -> Result<Self, String> {
        let problem = |error| format!("the run's XDG directories cannot be created: {error}");
        let dir = TempDir::new(label).map_err(problem)?;
        let mut paths = Vec::with_capacity(XDG.len());
        for (variable, name) in XDG {
            paths.push((variable, dir.create_dir(name).map_err(problem)?));
        }
        Ok(Self { _dir: dir, paths })
    }
}

/// The working directory and the standard input of a run. A run without a
/// working directory starts in the directory of the test that runs it, and
/// a run without standard input reads an empty one.
#[derive(Clone, Copy, Default)]
pub(crate) struct Place<'a> {
    /// The working directory, relative to the test's directory or absolute.
    pub(crate) dir: Option<&'a str>,
    /// The text on standard input.
    pub(crate) stdin: Option<&'a str>,
}

/// Runs `binary` once with `args`, a cleared environment, the variables of
/// `env`, the directories of `xdg` and the
/// working directory and standard input of `place`, and returns its output,
/// or the problem when the binary cannot start. A run that takes longer than
/// `timeout` is killed, and its output holds what it printed until then.
///
/// The checks own the process effects of each run, and run it through
/// snapbox's `cmd` module, which captures standard output and standard
/// error.
pub(crate) fn run(
    binary: &str,
    env: &[(&str, &str)],
    xdg: &Xdg,
    args: &[&str],
    place: Place<'_>,
    timeout: Duration,
) -> Result<Output, String> {
    let mut command = cmd::Command::new(binary)
        .args(args)
        .env_clear()
        .timeout(timeout);
    for &(name, value) in env {
        command = command.env(name, value);
    }
    for (variable, path) in &xdg.paths {
        command = command.env(variable, path);
    }
    if let Some(dir) = place.dir {
        command = command.current_dir(dir);
    }
    if let Some(stdin) = place.stdin {
        command = command.stdin(stdin.to_owned());
    }
    command
        .output()
        .map_err(|error| format!("the binary `{binary}` cannot start: {error}"))
}

/// Returns the failure that a run printed on `stdout` with `--json`: the
/// last line that is a `crumbkit/failure/v1` document, or the first entry of
/// the `failures` of the last line that has any, as a batch prints them.
pub(crate) fn failure(stdout: &[u8]) -> Option<Value> {
    String::from_utf8_lossy(stdout)
        .lines()
        .rev()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find_map(|mut value| {
            if value.get("schema").and_then(Value::as_str) == Some("crumbkit/failure/v1") {
                return Some(value);
            }
            let failures = value.get_mut("failures")?.as_array_mut()?;
            (!failures.is_empty()).then(|| failures.swap_remove(0))
        })
}
