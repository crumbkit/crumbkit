//! The runs of a tool's built binary that the example, command reference,
//! recovery and twin checks make, each with a cleared environment and fresh XDG directories.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::Duration;

use crumbkit::form::{FailureForm, ReadFailure, Step};
use serde_json::Value;
use snapbox::cmd;

use crate::temp::{TempDir, copy_tree};

/// The XDG base directory variables that each run sets, with the name of the
/// fresh directory that each one points at and the placeholder that the
/// example check writes in place of its path.
const XDG: [(&str, &str, &str); 4] = [
    ("XDG_CONFIG_HOME", "config", "[XDG_CONFIG_HOME]"),
    ("XDG_DATA_HOME", "data", "[XDG_DATA_HOME]"),
    ("XDG_STATE_HOME", "state", "[XDG_STATE_HOME]"),
    ("XDG_CACHE_HOME", "cache", "[XDG_CACHE_HOME]"),
];

/// How long a check lets one run take before it stops the run.
pub(crate) const TIMEOUT: Duration = Duration::from_secs(10);

/// Fresh XDG base directories, which one run or every run of one recovery
/// chain uses, so the runs of a chain see what the earlier runs wrote. The
/// directories are deleted when the value drops.
pub(crate) struct Xdg {
    /// The directory that holds the four XDG directories.
    _dir: TempDir,
    /// Each XDG variable with the path of its directory and its
    /// placeholder.
    paths: Vec<(&'static str, PathBuf, &'static str)>,
}

impl Xdg {
    /// Creates the four XDG directories in a new temporary directory whose
    /// name ends with `label`, and copies into each one the directory of
    /// the same name in `fixture`, such as `config` into the directory of
    /// `XDG_CONFIG_HOME`, or returns the problem when the directories cannot
    /// be created, the fixture breaks a rule of [`fixture_problem`] or a
    /// file cannot be copied.
    pub(crate) fn new(label: &str, fixture: Option<&str>) -> Result<Self, String> {
        let problem = |error| format!("the run's XDG directories cannot be created: {error}");
        if let Some(problem) = fixture.and_then(fixture_problem) {
            return Err(problem);
        }
        let dir = TempDir::new(label).map_err(problem)?;
        let mut paths = Vec::with_capacity(XDG.len());
        for (variable, name, placeholder) in XDG {
            let path = dir.create_dir(name).map_err(problem)?;
            if let Some(fixture) = fixture {
                let from = Path::new(fixture).join(name);
                if from.is_dir() {
                    copy_tree(&from, &path).map_err(|error| {
                        format!("the XDG fixture `{fixture}` cannot be copied: {error}")
                    })?;
                }
            }
            paths.push((variable, path, placeholder));
        }
        Ok(Self { _dir: dir, paths })
    }

    /// Returns each directory's placeholder, such as `[XDG_CONFIG_HOME]`,
    /// with its path.
    pub(crate) fn placeholders(&self) -> impl Iterator<Item = (&'static str, &PathBuf)> {
        self.paths
            .iter()
            .map(|(_, path, placeholder)| (*placeholder, path))
    }
}

/// Returns the problem of the XDG fixture `fixture`, a directory whose
/// entries may only be the directories `config`, `data`, `state` and
/// `cache`, or `None` when it keeps that rule.
pub(crate) fn fixture_problem(fixture: &str) -> Option<String> {
    let Ok(entries) = fs::read_dir(fixture) else {
        return Some(format!("the XDG fixture `{fixture}` is not a directory"));
    };
    let mut names: Vec<String> = Vec::new();
    for entry in entries {
        let Ok(entry) = entry else {
            return Some(format!("the XDG fixture `{fixture}` cannot be read"));
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        let is_dir = entry.file_type().is_ok_and(|kind| kind.is_dir());
        if !is_dir || !XDG.iter().any(|(_, known, _)| *known == name) {
            names.push(name);
        }
    }
    names.sort();
    names.first().map(|name| {
        format!(
            "the XDG fixture `{fixture}` holds `{name}`, and a fixture holds only the directories config, data, state and cache"
        )
    })
}

/// The directory that a run starts in, as a tool's settings name it.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Dir<'a> {
    /// A directory that every run starts in, which
    /// [`Examples::current_dir`](crate::Examples::current_dir) or
    /// [`Failing::dir`](crate::Failing::dir) names.
    Fixed(&'a str),
    /// A fixture that each run, or each recovery chain, starts in a new
    /// copy of, which [`Examples::fresh_dir`](crate::Examples::fresh_dir)
    /// names.
    Fresh(&'a str),
}

/// The working directory of one run, or of every run of one recovery chain:
/// none, a fixed directory, or a new copy of a fixture, which is deleted
/// when the value drops.
pub(crate) struct WorkDir {
    /// The directory's path, or `None` for the test's directory.
    path: Option<String>,
    /// The new copy of a fixture, which owns the directory of `path`.
    fresh: Option<TempDir>,
}

impl WorkDir {
    /// Returns the working directory that `dir` names. For a fixture, it
    /// creates a new temporary directory whose name ends with `label` and
    /// copies the fixture into it, or returns the problem when the directory
    /// cannot be created or a file cannot be copied.
    pub(crate) fn new(label: &str, dir: Option<Dir<'_>>) -> Result<Self, String> {
        match dir {
            None => Ok(Self {
                path: None,
                fresh: None,
            }),
            Some(Dir::Fixed(path)) => Ok(Self {
                path: Some(path.to_owned()),
                fresh: None,
            }),
            Some(Dir::Fresh(fixture)) => {
                let problem = |error| {
                    format!(
                        "the fixture `{fixture}` of Examples::fresh_dir cannot be copied: {error}"
                    )
                };
                let temp = TempDir::new(label).map_err(problem)?;
                copy_tree(Path::new(fixture), temp.path()).map_err(problem)?;
                Ok(Self {
                    path: Some(temp.path().display().to_string()),
                    fresh: Some(temp),
                })
            }
        }
    }

    /// Returns the directory's path, or `None` for the test's directory.
    pub(crate) fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    /// Returns the paths that the example check replaces with `[ROOT]`: the
    /// path of a new copy and its canonical path, which differs where a
    /// symbolic link leads to the temporary directory, as `/var` does on
    /// macOS. A fixed directory and the test's directory have none.
    pub(crate) fn fresh_paths(&self) -> Vec<PathBuf> {
        let Some(temp) = &self.fresh else {
            return Vec::new();
        };
        let mut paths = vec![temp.path().to_path_buf()];
        if let Ok(canonical) = fs::canonicalize(temp.path())
            && canonical != temp.path()
        {
            paths.push(canonical);
        }
        paths
    }
}

/// The working directory and the standard input of a run. A run without a
/// working directory starts in the directory of the test that runs it, and
/// a run without standard input reads an empty one.
#[derive(Clone, Copy, Default)]
pub(crate) struct Place<'a> {
    /// The working directory, relative to the test's directory or absolute.
    pub(crate) dir: Option<&'a str>,
    /// The bytes on standard input.
    pub(crate) stdin: Option<&'a [u8]>,
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
    for (variable, path, _) in &xdg.paths {
        command = command.env(variable, path);
    }
    if let Some(dir) = place.dir {
        command = command.current_dir(dir);
    }
    if let Some(stdin) = place.stdin {
        command = command.stdin(stdin.to_vec());
    }
    command
        .output()
        .map_err(|error| format!("the binary `{binary}` cannot start: {error}"))
}

/// Returns the failure that a run printed on `stdout` with `--json`: the
/// last `crumbkit/failure/v1` document, or the first entry of the
/// `failures` of the last document that has any, as a batch prints them.
/// It reads documents on one line and indented documents alike.
pub(crate) fn failure(stdout: &[u8]) -> Option<Value> {
    documents(&String::from_utf8_lossy(stdout))
        .into_iter()
        .rev()
        .find_map(|mut value| {
            if value.get("schema").and_then(Value::as_str) == Some("crumbkit/failure/v1") {
                return Some(value);
            }
            let failures = value.get_mut("failures")?.as_array_mut()?;
            (!failures.is_empty()).then(|| failures.swap_remove(0))
        })
}

/// Returns the `fields` of the failure that [`failure`] finds on `stdout`,
/// each a name and its value as text, in order. A string is its own text,
/// an integer or a boolean is written as JSON writes it, and a field of
/// another type is left out.
pub(crate) fn fields(stdout: &[u8]) -> Vec<(String, String)> {
    let Some(document) = failure(stdout) else {
        return Vec::new();
    };
    let Some(fields) = document.get("fields").and_then(Value::as_object) else {
        return Vec::new();
    };
    fields
        .iter()
        .filter_map(|(name, value)| {
            let text = match value {
                Value::String(text) => text.clone(),
                Value::Number(_) | Value::Bool(_) => value.to_string(),
                _ => return None,
            };
            Some((name.clone(), text))
        })
        .collect()
}

/// Returns the failure that a failed run printed on `stdout` and `stderr`:
/// what `form` reads back when the guide sets a form and its
/// [`FailureForm::read`] finds one, and otherwise the code and steps of
/// crumbkit's own document, which [`failure`] finds on `stdout`.
pub(crate) fn read_failure(
    form: Option<&dyn FailureForm>,
    stdout: &[u8],
    stderr: &[u8],
) -> Option<ReadFailure> {
    if let Some(form) = form {
        let read = form.read(
            &String::from_utf8_lossy(stdout),
            &String::from_utf8_lossy(stderr),
        );
        if read.is_some() {
            return read;
        }
    }
    failure(stdout).as_ref().and_then(read_document)
}

/// Returns the code of the failure document `document` and its steps, the
/// recovery first and then each alternative, or `None` when it has no code
/// or no recovery.
fn read_document(document: &Value) -> Option<ReadFailure> {
    let code = document.get("code")?.as_str()?.to_owned();
    let alternatives = document
        .get("alternatives")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice);
    let text = |step: &Value, key: &str| {
        step.get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let steps = std::iter::once(document.get("recovery")?)
        .chain(alternatives)
        .map(|step| match step.get("kind").and_then(Value::as_str) {
            Some("run") => step.get("call").and_then(Value::as_str).map_or_else(
                || Step::None(String::new()),
                |call| Step::Run(call.to_owned()),
            ),
            Some("retry") => Step::Retry,
            Some("hint") => Step::Hint(text(step, "text")),
            _ => Step::None(text(step, "reason")),
        })
        .collect();
    Some(ReadFailure::new(code, steps))
}

/// Returns the code of the last `code:` line in `stderr`, which crumbkit's
/// text form of a failure ends with.
pub(crate) fn text_code(stderr: &[u8]) -> Option<String> {
    String::from_utf8_lossy(stderr)
        .lines()
        .rev()
        .find_map(|line| line.trim_start().strip_prefix("code:"))
        .map(|code| code.trim().to_owned())
}

/// Returns the code of `failure` and its steps as one phrase, such as
/// `tool::books::not_found, whose steps are: run `tool books list`; retry`.
pub(crate) fn describe(failure: &ReadFailure) -> String {
    let steps: Vec<String> = failure
        .steps()
        .iter()
        .map(|step| match step {
            Step::Run(call) => format!("run `{call}`"),
            Step::Retry => "retry".to_owned(),
            Step::Hint(text) => format!("hint: {text}"),
            Step::None(reason) if reason.is_empty() => "no next step".to_owned(),
            Step::None(reason) => format!("no next step: {reason}"),
            _ => "a step that crumbkit-test cannot name".to_owned(),
        })
        .collect();
    if steps.is_empty() {
        failure.code().to_owned()
    } else {
        format!("{}, whose steps are: {}", failure.code(), steps.join("; "))
    }
}

/// Returns each JSON document in `stdout` in order, whether it fills one
/// line or spans several lines indented. A line where no document starts,
/// such as a line of text, is skipped.
pub(crate) fn documents(stdout: &str) -> Vec<Value> {
    let mut found = Vec::new();
    let mut rest = stdout;
    while !rest.trim_start().is_empty() {
        let mut stream = serde_json::Deserializer::from_str(rest).into_iter::<Value>();
        if let Some(Ok(value)) = stream.next() {
            found.push(value);
            rest = &rest[stream.byte_offset()..];
        } else {
            rest = rest.split_once('\n').map_or("", |(_, after)| after);
        }
    }
    found
}

#[cfg(test)]
mod tests;
