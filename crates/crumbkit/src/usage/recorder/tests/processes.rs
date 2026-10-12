//! The usage log's tests that run this test binary again as a child
//! process, which starts the log and raises a signal in itself.

use std::fs;

use serde_json::Value;

use crate::usage::testing::TempDir;

// Each platform raises its own signals and runs its children through its
// own shell, so each has a file of tests.
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

/// The variable that tells a signal test's child process to run.
const CHILD: &str = "CRUMBKIT_USAGE_SIGNAL_CHILD";

/// Runs the test named `test` of this test binary as a child process with
/// [`CHILD`] set and the state directory `state`, and returns how it ended.
/// On Unix the child runs in a process group of its own, so a signal that
/// it sends to its process group never reaches the test runner.
fn run_child(test: &str, state: &TempDir) -> std::process::ExitStatus {
    #[expect(
        clippy::disallowed_methods,
        reason = "a signal test runs this test binary again as the child that receives the signal"
    )]
    let mut child = std::process::Command::new(std::env::current_exe().unwrap());
    child
        .args(["--exact", test, "--nocapture"])
        .env(CHILD, "1")
        .env("XDG_STATE_HOME", state.path());
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut child, 0);
    child.status().unwrap()
}

/// Returns the lines of the log of the tool `tool` under `state`.
fn log_lines(state: &TempDir, tool: &str) -> Vec<Value> {
    let log = fs::read_to_string(state.path().join(tool).join("usage.jsonl")).unwrap();
    log.lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

/// Reports whether this process runs as the child of a signal test.
fn is_child() -> bool {
    #[expect(
        clippy::disallowed_methods,
        reason = "a signal test tells its child process to run through the environment"
    )]
    std::env::var_os(CHILD).is_some()
}
