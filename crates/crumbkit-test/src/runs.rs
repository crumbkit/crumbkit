//! The runs of a tool's built binary that the example, command reference,
//! recovery and twin checks make, each with a cleared environment and fresh XDG directories.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

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
    /// The variables of a failing line, each set to its value or removed
    /// for `None`, which win over every other variable of the run.
    pub(crate) env: &'a [(&'a str, Option<&'a str>)],
}

/// How a run of the binary ended.
pub(crate) enum Ran {
    /// The run exited on its own, with what it printed and its exit status.
    Exited(Output),
    /// The run did not end within its timeout, so the check killed it.
    Stopped,
}

/// Returns the problem of a run that the check stopped after `timeout`.
pub(crate) fn stopped(timeout: Duration) -> String {
    format!(
        "the run did not end within {} seconds, so the check stopped it",
        timeout.as_secs_f64()
    )
}

/// Runs `binary` once with `args`, a cleared environment, the variables of
/// `env`, the directories of `xdg` and the working directory, standard
/// input and variables of `place`, which it applies last, and returns how the run ended, or the problem when the
/// binary cannot start or the check cannot wait for it. The run's `timeout`
/// starts when the binary starts. A run that has not exited when `timeout`
/// ends is killed and returns [`Ran::Stopped`], so a caller learns of a
/// timeout from the run itself.
///
/// The checks own the process effects of each run. One thread writes the
/// standard input and then closes it, and two threads read standard output
/// and standard error while the run goes on, so a run that never reads more
/// input than a pipe holds, or prints more than a pipe holds, does not block
/// the check. After the run exits, the check waits for the two readers
/// until `timeout` ends. A process that the run started and that still
/// holds standard output or standard error open then makes the run a
/// problem, so the check does not wait for that process to end.
pub(crate) fn run(
    binary: &str,
    env: &[(&str, &str)],
    xdg: &Xdg,
    args: &[&str],
    place: Place<'_>,
    timeout: Duration,
) -> Result<Ran, String> {
    #[expect(
        clippy::disallowed_methods,
        reason = "the checks own the runs of a tool's binary"
    )]
    let mut command = Command::new(binary);
    command
        .args(args)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for &(name, value) in env {
        command.env(name, value);
    }
    for (variable, path, _) in &xdg.paths {
        command.env(variable, path);
    }
    for &(name, value) in place.env {
        match value {
            Some(value) => command.env(name, value),
            None => command.env_remove(name),
        };
    }
    if let Some(dir) = place.dir {
        command.current_dir(dir);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("the binary `{binary}` cannot start: {error}"))?;
    let started = Instant::now();
    if let Some(pipe) = child.stdin.take() {
        write_all(pipe, place.stdin.unwrap_or_default().to_vec());
    }
    let stdout = child.stdout.take().map(read_all);
    let stderr = child.stderr.take().map(read_all);
    let waited = wait(&mut child, started, timeout)
        .map_err(|error| format!("the run of `{binary}` cannot be waited for: {error}"))?;
    let Some(status) = waited else {
        return Ok(Ran::Stopped);
    };
    let held = || {
        format!(
            "the run exited, and a process that it started still held its standard output or standard error open when its timeout of {} seconds ended",
            timeout.as_secs_f64()
        )
    };
    let stdout = collect(stdout, started, timeout).ok_or_else(held)?;
    let stderr = collect(stderr, started, timeout).ok_or_else(held)?;
    Ok(Ran::Exited(Output {
        status,
        stdout,
        stderr,
    }))
}

/// The longest pause between two checks of whether a run has exited.
const LONGEST_PAUSE: Duration = Duration::from_millis(10);

/// Waits for `child` to exit and returns its exit status, or kills it and
/// returns `None` when it has not exited `timeout` after `started`. It checks after
/// 1 ms and then doubles the pause up to [`LONGEST_PAUSE`], so a short run
/// waits little and a long run costs few checks. When the check cannot
/// learn whether `child` exited, it kills `child` before it returns the
/// error, so no run outlives the check.
fn wait(child: &mut Child, started: Instant, timeout: Duration) -> io::Result<Option<ExitStatus>> {
    let mut pause = Duration::from_millis(1);
    loop {
        let exited = match child.try_wait() {
            Ok(exited) => exited,
            Err(error) => {
                // The run may already have exited, so a failed kill is not
                // a second error.
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        };
        if let Some(status) = exited {
            return Ok(Some(status));
        }
        let left = timeout.saturating_sub(started.elapsed());
        if left.is_zero() {
            // RUST-10: the kill stops the run's own process only. A process
            // that the run started survives the kill and can keep running
            // after the check ends. Ending those processes too needs a
            // signal sent to the run's process group on Unix, or a job
            // object on Windows. The standard library offers neither, so a
            // check that must end them adds a crate that does.
            child.kill()?;
            child.wait()?;
            return Ok(None);
        }
        thread::sleep(pause.min(left));
        pause = (pause * 2).min(LONGEST_PAUSE);
    }
}

/// Writes `bytes` to `pipe` on a new thread and then drops the pipe, which
/// closes the run's standard input. Nothing waits for the thread: a run may
/// exit or be killed without reading its input, so a failed write, such as
/// a broken pipe, is not a problem of the check.
fn write_all(mut pipe: impl Write + Send + 'static, bytes: Vec<u8>) {
    thread::spawn(move || {
        let _ = pipe.write_all(&bytes);
    });
}

/// Reads everything from `pipe` on a new thread, which sends the bytes on
/// the returned channel when the pipe closes.
fn read_all(mut pipe: impl Read + Send + 'static) -> Receiver<Vec<u8>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut bytes = Vec::new();
        // A read that fails keeps the bytes read until then.
        let _ = pipe.read_to_end(&mut bytes);
        // The check stops listening when the timeout ends, so a failed send
        // is not a problem.
        let _ = sender.send(bytes);
    });
    receiver
}

/// Returns the bytes that `reader` read, or none when there is no reader or
/// its thread panicked. It returns `None` when the pipe is still open
/// `timeout` after `started`.
fn collect(
    reader: Option<Receiver<Vec<u8>>>,
    started: Instant,
    timeout: Duration,
) -> Option<Vec<u8>> {
    let Some(reader) = reader else {
        return Some(Vec::new());
    };
    match reader.recv_timeout(timeout.saturating_sub(started.elapsed())) {
        Ok(bytes) => Some(bytes),
        Err(mpsc::RecvTimeoutError::Disconnected) => Some(Vec::new()),
        Err(mpsc::RecvTimeoutError::Timeout) => None,
    }
}

/// Returns the note that follows a command line whose run sets or removes
/// the variables of `env`, such as ` (with DEPTH=1, HOME removed)`, or an
/// empty note when `env` is empty.
pub(crate) fn env_note(env: &[(&str, Option<&str>)]) -> String {
    if env.is_empty() {
        return String::new();
    }
    let entries: Vec<String> = env
        .iter()
        .map(|&(name, value)| match value {
            Some(value) => format!("{name}={value}"),
            None => format!("{name} removed"),
        })
        .collect();
    format!(" (with {})", entries.join(", "))
}

/// Returns `words` with `--json` added when they lack it: before the first
/// `--`, after which a tool reads every word as a positional value, or at
/// the end of a line without one.
pub(crate) fn with_json(words: &[String]) -> Vec<String> {
    let mut args = words.to_vec();
    if !args.iter().any(|word| word == "--json") {
        let at = args
            .iter()
            .position(|word| word == "--")
            .unwrap_or(args.len());
        args.insert(at, "--json".to_owned());
    }
    args
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
