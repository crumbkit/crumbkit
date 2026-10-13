//! The usage log's tests that run this test binary again as a child
//! process, which starts the log and raises a signal in itself.

use std::fs;

use clap::Command;
use serde_json::Value;

use crate::help::{Exit, Guide};
use crate::usage::recorder::{start, typed};
use crate::usage::testing::TempDir;
use crate::{Code, Document, Output, Parsed, Sink, Spelling, Stream, stream_to};

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

#[test]
fn a_stream_that_ends_with_a_mapped_failure_records_the_declared_exit_and_its_bytes() {
    let state = TempDir::new("stream-exit");

    let status = run_child(
        "usage::recorder::tests::processes::the_stream_exit_test_s_child_streams_one_row_and_times_out",
        &state,
    );

    assert_eq!(status.code(), Some(124));
    let lines = log_lines(&state, "watching");
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["exit"], 124);
    assert_eq!(lines[0]["code"], "watching::watch::timed_out");
    assert_eq!(lines[0]["out_bytes"], "NUMBER\n1\n".len());
}

/// The code of the failure that the stream-exit test's child ends with,
/// which its guide maps to 124.
const TIMED_OUT: Code = Code::new("watching", "watch", "timed_out");

/// A row of the stream-exit test's child.
#[derive(serde::Serialize, schemars::JsonSchema)]
struct Numbered {
    number: u32,
}

impl crate::Row for Numbered {
    const COLUMNS: &'static [&'static str] = &["number"];
}

/// The failure that the stream-exit test's child ends with.
#[derive(Debug)]
struct TimedOut;

impl std::fmt::Display for TimedOut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the watch ran out of time")
    }
}

impl std::error::Error for TimedOut {}

impl crate::Failure for TimedOut {
    fn code(&self) -> &'static Code {
        &TIMED_OUT
    }

    fn recovery(&self) -> crate::Recovery {
        crate::Recovery::Retry
    }
}

/// Runs only as the child of the stream-exit test, which sets [`CHILD`]. It
/// starts the log, streams one row with a guide that maps `TIMED_OUT` to
/// 124, ends the stream with that failure, and exits with the code that the
/// stream returns.
#[test]
fn the_stream_exit_test_s_child_streams_one_row_and_times_out() {
    const GUIDE: Guide =
        Guide::new().exits(&[Exit::new(124, "The watch ran out of time.").failures(&[&TIMED_OUT])]);
    const NUMBERS: Document<Stream<Numbered>> =
        Document::new("watching", "numbers", 1, "Each number as it arrives.");
    if !is_child() {
        return;
    }
    let log = start();
    let command = Command::new("watching");
    typed(&command, &["watching".into()], Spelling::COLONS);
    let mut output = Output::default();
    output.set_command(Parsed::guided(command, &GUIDE));
    let mut sink = Sink::new(Vec::new(), Vec::new());
    let mut streaming = stream_to(&mut sink, &mut output, Spelling::COLONS, NUMBERS).unwrap();
    let _ = streaming.item(&Numbered { number: 1 });
    let code = streaming.end(Err(TimedOut));
    drop(log);
    #[expect(
        clippy::disallowed_methods,
        reason = "the child ends with the stream's exit code, which the test reads from its status"
    )]
    std::process::exit(if code == std::process::ExitCode::from(124) {
        124
    } else {
        1
    });
}
