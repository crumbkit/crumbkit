//! The temporary directories that the usage log's unit tests write in, the
//! command tree that the report's unit tests check records against, and the
//! log lines that they read.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use clap::{Arg, ArgAction, Command};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicUsize, Ordering};

/// The number of temporary directories that this process has created, which
/// gives each new directory a number of its own.
static CREATED: AtomicUsize = AtomicUsize::new(0);

/// A directory under the system's temporary directory, which is deleted with
/// everything in it when the value drops. Its name holds the process id and
/// a number, so tests at the same time never share a directory. The
/// directory does not exist until a test creates it.
pub(super) struct TempDir {
    /// The directory's path.
    path: PathBuf,
}

impl TempDir {
    /// Returns a new directory path that ends with `label`.
    pub(super) fn new(label: &str) -> Self {
        let number = CREATED.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "crumbkit-usage-{}-{number}-{label}",
            std::process::id()
        ));
        Self { path }
    }

    /// Returns the directory's path.
    pub(super) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        #[expect(
            clippy::disallowed_methods,
            reason = "the temporary directory owns its path, and deletes it when the value drops"
        )]
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Returns a built tool named shelf with a global `--full`, `books list`
/// with `--all`, `--limit` and `-n`, and `books show` with ids.
pub(super) fn tool() -> Command {
    let flag = |name: &'static str| Arg::new(name).long(name).action(ArgAction::SetTrue);
    let mut tool = Command::new("shelf")
        .arg(flag("full").global(true))
        .subcommand(
            Command::new("books")
                .subcommand(
                    Command::new("list")
                        .alias("ls")
                        .arg(flag("all"))
                        .arg(Arg::new("limit").long("limit"))
                        .arg(Arg::new("number").short('n')),
                )
                .subcommand(Command::new("show").arg(Arg::new("ids").num_args(1..))),
        );
    tool.build();
    tool
}

/// The time that the tests report at: noon UTC on 11 October 2026.
pub(super) fn now() -> SystemTime {
    humantime::parse_rfc3339("2026-10-11T12:00:00Z").unwrap()
}

/// Returns one line of the log for a person's run of `command` in session
/// `s1` that ended `secs` seconds before [`now`], with `fields` added.
pub(super) fn line(secs: u64, command: &str, fields: &Value) -> String {
    let at = humantime::format_rfc3339_millis(now() - Duration::from_secs(secs));
    let mut record = json!({
        "schema": "crumbkit/usage/v1",
        "tool": "shelf",
        "at": at.to_string(),
        "ms": 10,
        "caller": "person",
        "session": "0123456789abcdef",
        "args": format!("{secs:016x}"),
        "trace": "4bf92f3577b34da6a3ce929d0e0e4736",
        "run": "00f067aa0ba902b7",
        "exit": 0,
        "out_bytes": 100,
    });
    if !command.is_empty() {
        record["command"] = json!(command);
    }
    for (field, value) in fields.as_object().unwrap() {
        record[field] = value.clone();
    }
    format!("{record}\n")
}
