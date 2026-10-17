//! The usage log's tests that run this test binary again as a child
//! process, which starts the log and raises a signal in itself.

use std::fs;

use clap::Command;
use serde_json::Value;

use crate::recorder::start;
use crate::testing::TempDir;
use crumbkit::help::{Exit, Guide};
use crumbkit::{Code, Document, Output, Parsed, Sink, Spelling, Stream, stream_to};

/// Tells the usage log that the tool `command` received `args`, as
/// crumbkit's `parse` does, so the line has a tool name.
fn typed(command: &Command, args: &[std::ffi::OsString], spelling: Spelling) {
    super::super::report(|recorder| recorder.typed(command, args, spelling));
}

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
        "recorder::tests::processes::the_stream_exit_test_s_child_streams_one_row_and_times_out",
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

impl crumbkit::Row for Numbered {
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

impl crumbkit::Failure for TimedOut {
    fn code(&self) -> &'static Code {
        &TIMED_OUT
    }

    fn recovery(&self) -> crumbkit::Recovery {
        crumbkit::Recovery::Retry
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

#[test]
fn a_start_after_the_tool_installed_its_own_observer_writes_the_line_and_both_hear_each_report() {
    let state = TempDir::new("own-observer");

    let status = run_child(
        "recorder::tests::processes::the_own_observer_test_s_child_installs_an_observer_and_then_starts_the_log",
        &state,
    );

    assert_eq!(status.code(), Some(0));
    let lines = log_lines(&state, "owned");
    assert_eq!(lines.len(), 1);
    assert_eq!(
        lines[0]["tolerated"],
        serde_json::json!(["owned::cache::stale"])
    );
}

/// The code that the own-observer test's child tolerates.
const STALE: Code = Code::new("owned", "cache", "stale");

/// An observer of the tool's own, which notes that it heard a tolerated
/// code.
struct Quiet(std::sync::atomic::AtomicBool);

impl crumbkit::events::Observer for Quiet {
    fn tolerated(&self, _code: &'static Code) {
        self.0.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

/// The own-observer test's observer.
static QUIET: Quiet = Quiet(std::sync::atomic::AtomicBool::new(false));

/// Runs only as the child of the own-observer test. It installs [`QUIET`],
/// starts the log, gives the recorder a tool name and reports a tolerated
/// code. It exits with 3 when [`QUIET`] did not hear the code.
#[test]
fn the_own_observer_test_s_child_installs_an_observer_and_then_starts_the_log() {
    if !is_child() {
        return;
    }
    assert!(crumbkit::events::observe(&QUIET));
    let log = start();
    typed(&Command::new("owned"), &["owned".into()], Spelling::COLONS);
    crumbkit::events::tolerated(&STALE);
    drop(log);
    if !QUIET.0.load(std::sync::atomic::Ordering::SeqCst) {
        #[expect(
            clippy::disallowed_methods,
            reason = "the child tells the test that its own observer missed the report through its exit code"
        )]
        std::process::exit(3);
    }
}

#[test]
fn starting_the_log_again_installs_its_observer_once() {
    let state = TempDir::new("starts");

    let status = run_child(
        "recorder::tests::processes::the_starts_test_s_child_starts_the_log_four_times_and_installs_three_observers",
        &state,
    );

    assert_eq!(status.code(), Some(0));
}

/// An observer that hears nothing, which the starts test's child installs
/// three times.
struct Silent;

impl crumbkit::events::Observer for Silent {}

static SILENT: Silent = Silent;

/// Runs only as the child of the starts test. It starts the log four times,
/// and then installs three more observers, which fit only when the log took
/// one of crumbkit's four observer slots.
#[test]
fn the_starts_test_s_child_starts_the_log_four_times_and_installs_three_observers() {
    if !is_child() {
        return;
    }
    let logs = [start(), start(), start(), start()];
    for _ in 0..3 {
        assert!(crumbkit::events::observe(&SILENT));
    }
    drop(logs);
}
