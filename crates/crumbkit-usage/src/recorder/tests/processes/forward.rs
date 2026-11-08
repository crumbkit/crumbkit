//! The tests of the usage log beside crumbkit-process's forwarder. Each test
//! runs this test binary again as the tool, in a process group of its own,
//! so the signal that the tool raises ends the tool and never the test
//! runner.

use std::fs;
use std::os::unix::process::ExitStatusExt as _;
use std::path::PathBuf;
use std::process::{Command as Process, ExitStatus};
use std::time::{Duration, Instant};

use clap::Command;
use crumbkit::Spelling;
use crumbkit_process::forward;
use signal_hook::consts::{SIGHUP, SIGTERM};

use super::{is_child, log_lines, run_child, typed};
use crate::recorder::{Log, start_under};
use crate::testing::TempDir;

/// How long a process the forwarder killed may take to be reaped by its new
/// parent. A group that the forwarder never killed sleeps for 30 seconds, so
/// it outlasts this.
const REAP: Duration = Duration::from_secs(2);

/// Returns the state directory that the parent test gave this tool.
fn state_home() -> PathBuf {
    #[expect(
        clippy::disallowed_methods,
        reason = "the tool finds the directory that its test named through the environment"
    )]
    std::env::var_os("XDG_STATE_HOME").unwrap().into()
}

/// Returns a command that runs `script` with `sh`.
fn sh(script: &str) -> Process {
    #[expect(
        clippy::disallowed_methods,
        reason = "the test builds the command that the run starts"
    )]
    let mut sh = Process::new("sh");
    sh.args(["-c", script]);
    sh
}

/// Reports whether the process `pid` is gone within [`REAP`].
fn gone(pid: i32) -> bool {
    let deadline = Instant::now() + REAP;
    loop {
        if !sh(&format!("kill -0 {pid} 2>/dev/null"))
            .status()
            .unwrap()
            .success()
        {
            return true;
        }
        if Instant::now() > deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Runs the test named `test` of this module as the tool.
fn run_tool(test: &str, state: &TempDir) -> ExitStatus {
    run_child(
        &format!("recorder::tests::processes::forward::{test}"),
        state,
    )
}

/// Starts the log and the forwarder as an adopter's `main` does, and names
/// the tool `tool`.
fn start_tool(tool: &str) -> Log {
    let log = start_under(forward::clean_up);
    forward::install().unwrap();
    typed(
        &Command::new(tool.to_owned()),
        &[tool.into()],
        Spelling::COLONS,
    );
    log
}

/// Raises `signal` on this tool and waits for the signal to end it.
fn raise_and_wait(signal: i32) {
    signal_hook::low_level::raise(signal).unwrap();
    std::thread::sleep(Duration::from_secs(10));
    panic!("the signal did not end the tool");
}

/// Starts a run whose child ignores SIGTERM and has written its process id
/// to the file `pid` under the state directory.
fn start_child_that_ignores_sigterm() {
    #[expect(
        clippy::disallowed_methods,
        reason = "the test creates the directory that its child writes the process id to"
    )]
    fs::create_dir_all(state_home()).unwrap();
    let pid = state_home().join("pid");
    let script = format!(
        "trap '' TERM; echo $$ > '{}.tmp'; mv '{0}.tmp' '{0}'; exec sleep 30",
        pid.display()
    );
    #[expect(
        clippy::default_trait_access,
        reason = "the test wants a run's default settings and does not depend on the name of their type"
    )]
    let _runner =
        std::thread::spawn(move || crumbkit_process::run(sh(&script), &Default::default()));
    let deadline = Instant::now() + Duration::from_secs(10);
    while !pid.exists() {
        assert!(Instant::now() < deadline, "the child wrote no process id");
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Returns the process id that the child wrote.
fn child_pid(state: &TempDir) -> i32 {
    fs::read_to_string(state.path().join("pid"))
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}

#[test]
fn a_signal_kills_a_group_that_ignores_it_before_the_tool_ends_and_writes_the_line() {
    if is_child() {
        let _log = start_tool("trapped");
        start_child_that_ignores_sigterm();
        raise_and_wait(SIGTERM);
    }
    let state = TempDir::new("forward-kill");

    let status = run_tool(
        "a_signal_kills_a_group_that_ignores_it_before_the_tool_ends_and_writes_the_line",
        &state,
    );

    assert_eq!(
        status.signal(),
        Some(SIGTERM),
        "the tool ended with {status:?}"
    );
    let child = child_pid(&state);
    assert!(gone(child), "the child {child} outlived the tool");
    let lines = log_lines(&state, "trapped");
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["interrupted"], true);
}

#[test]
fn a_signal_with_no_live_group_still_writes_the_line() {
    if is_child() {
        let _log = start_tool("idle");
        raise_and_wait(SIGTERM);
    }
    for round in 0..25 {
        let state = TempDir::new("forward-idle");

        let status = run_tool("a_signal_with_no_live_group_still_writes_the_line", &state);

        assert_eq!(status.signal(), Some(SIGTERM), "round {round}: {status:?}");
        let lines = log_lines(&state, "idle");
        assert_eq!(lines.len(), 1, "round {round}");
        assert_eq!(lines[0]["interrupted"], true, "round {round}");
    }
}

#[test]
fn sighup_writes_the_line_and_ends_the_tool_by_sighup() {
    if is_child() {
        let _log = start_tool("hung-up");
        raise_and_wait(SIGHUP);
    }
    let state = TempDir::new("forward-hup");

    let status = run_tool("sighup_writes_the_line_and_ends_the_tool_by_sighup", &state);

    assert_eq!(
        status.signal(),
        Some(SIGHUP),
        "the tool ended with {status:?}"
    );
    let lines = log_lines(&state, "hung-up");
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["interrupted"], true);
}

/// The tool's own clean-up, which copies the log so the test can see what
/// the log held when this clean-up ran.
fn copy_the_log() {
    let state = state_home();
    let _ = fs::copy(state.join("ordered/usage.jsonl"), state.join("copy.jsonl"));
}

#[test]
fn a_later_clean_up_finds_the_line_written() {
    if is_child() {
        let _log = start_tool("ordered");
        forward::clean_up(copy_the_log);
        raise_and_wait(SIGTERM);
    }
    let state = TempDir::new("forward-order");

    let status = run_tool("a_later_clean_up_finds_the_line_written", &state);

    assert_eq!(
        status.signal(),
        Some(SIGTERM),
        "the tool ended with {status:?}"
    );
    let copy = fs::read_to_string(state.path().join("copy.jsonl")).unwrap();
    let line: serde_json::Value = serde_json::from_str(copy.trim()).unwrap();
    assert_eq!(line["interrupted"], true);
}
