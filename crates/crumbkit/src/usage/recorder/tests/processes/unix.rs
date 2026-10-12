//! The signal tests on Unix, where a test raises SIGTERM or SIGINT, and a
//! child that crumbkit hands the terminal to runs through `sh`.

use std::os::unix::process::ExitStatusExt as _;
#[cfg(feature = "hand-over")]
use std::process::ExitCode;
use std::time::{Duration, Instant};

use clap::Command;

use super::{is_child, log_lines, run_child};
use crate::Spelling;
use crate::usage::recorder::{RECORDER, Signals, State, ended, start, start_with, typed};
use crate::usage::testing::TempDir;

#[test]
fn a_signal_writes_an_interrupted_line_and_then_ends_the_process_as_the_signal_does() {
    let state = TempDir::new("signal");

    let status = run_child(
        "usage::recorder::tests::processes::unix::the_signal_test_s_child_starts_the_log_and_raises_sigterm",
        &state,
    );

    assert_eq!(status.signal(), Some(signal_hook::consts::SIGTERM));
    let lines = log_lines(&state, "signalled");
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["tool"], "signalled");
    assert_eq!(lines[0]["interrupted"], true);
}

/// Runs only as the child of the signal test, which sets [`CHILD`]. It
/// starts the log, gives it the tool's name as `parse` does, and raises
/// SIGTERM, which the log's thread answers.
#[test]
fn the_signal_test_s_child_starts_the_log_and_raises_sigterm() {
    if !is_child() {
        return;
    }
    let _log = start();
    typed(
        &Command::new("signalled"),
        &["signalled".into()],
        Spelling::COLONS,
    );
    signal_hook::low_level::raise(signal_hook::consts::SIGTERM).unwrap();
    // The signal ends the process, so the test never gets past the wait.
    std::thread::sleep(Duration::from_secs(10));
    panic!("SIGTERM did not end the process");
}

#[test]
fn a_tool_that_owns_its_signals_keeps_running_after_sigterm_and_reports_its_own_exit() {
    let state = TempDir::new("owned");

    let status = run_child(
        "usage::recorder::tests::processes::unix::the_owned_signal_test_s_child_raises_sigterm_and_ends_with_124",
        &state,
    );

    assert!(status.success());
    let lines = log_lines(&state, "owner");
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["interrupted"], true);
    assert_eq!(lines[0]["exit"], 124);
    assert_eq!(lines[0]["out_bytes"], 5);
}

/// Runs only as the child of the owned-signal test, which sets [`CHILD`].
/// It starts the log for a tool that owns its signals, raises SIGTERM,
/// waits until the log's thread has recorded it, and reports exit 124.
#[test]
fn the_owned_signal_test_s_child_raises_sigterm_and_ends_with_124() {
    if !is_child() {
        return;
    }
    let _log = start_with(Signals::Tool);
    typed(&Command::new("owner"), &["owner".into()], Spelling::COLONS);
    signal_hook::low_level::raise(signal_hook::consts::SIGTERM).unwrap();
    let waited = Instant::now();
    while !matches!(&*RECORDER.lock().unwrap(), State::Recording(recorder) if recorder.signalled) {
        assert!(
            waited.elapsed() < Duration::from_secs(10),
            "the signal was not recorded"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    ended(124, Some(5));
}

#[cfg(feature = "hand-over")]
#[test]
fn ctrl_c_while_a_child_holds_the_terminal_ends_only_the_child_and_the_line_records_it() {
    let state = TempDir::new("hand-over");

    let status = run_child(
        "usage::recorder::tests::processes::unix::the_hand_over_test_s_child_hands_the_terminal_to_a_child_that_ctrl_c_ends",
        &state,
    );

    assert_eq!(status.code(), Some(130), "the tool ended with {status:?}");
    let lines = log_lines(&state, "editing");
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["interrupted"], true);
    assert_eq!(lines[0]["exit"], 130);
}

/// Runs only as the child of the hand-over test, which sets [`CHILD`]. It
/// starts the log, which listens for SIGINT, and hands the terminal to a
/// shell that sends SIGINT to its process group, as Ctrl-C does, and ends
/// by it.
#[cfg(feature = "hand-over")]
#[test]
fn the_hand_over_test_s_child_hands_the_terminal_to_a_child_that_ctrl_c_ends() {
    if !is_child() {
        return;
    }
    let _log = start();
    typed(
        &Command::new("editing"),
        &["editing".into()],
        Spelling::COLONS,
    );
    #[expect(
        clippy::disallowed_methods,
        reason = "the test builds the command that crumbkit hands the terminal to"
    )]
    let mut shell = std::process::Command::new("sh");
    shell.args(["-c", "kill -INT 0; sleep 5"]);
    let code = crate::child::hand_over(&mut crate::Output::default(), Spelling::COLONS, shell);
    let number = if code == ExitCode::from(130) { 130 } else { 1 };
    #[expect(
        clippy::disallowed_methods,
        reason = "the child of the hand-over test reports the tool's exit code through its own"
    )]
    std::process::exit(number);
}

#[cfg(feature = "hand-over")]
#[test]
fn a_tool_that_owns_its_signals_keeps_running_on_ctrl_c_after_a_child_has_held_the_terminal() {
    let state = TempDir::new("owned-hand-over");

    let status = run_child(
        "usage::recorder::tests::processes::unix::the_owned_hand_over_test_s_child_raises_sigint_after_its_child_exits",
        &state,
    );

    assert_eq!(status.code(), Some(0), "the tool ended with {status:?}");
    let lines = log_lines(&state, "owner");
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["exit"], 0);
}

/// Runs only as the child of the owned hand-over test, which sets
/// [`CHILD`]. It starts the log for a tool that owns its signals, hands the
/// terminal to a shell that exits at once, which writes the line, then
/// raises SIGINT, which the tool owns, and ends with exit 0 if it is still
/// running.
#[cfg(feature = "hand-over")]
#[test]
fn the_owned_hand_over_test_s_child_raises_sigint_after_its_child_exits() {
    if !is_child() {
        return;
    }
    let _log = start_with(Signals::Tool);
    typed(&Command::new("owner"), &["owner".into()], Spelling::COLONS);
    #[expect(
        clippy::disallowed_methods,
        reason = "the test builds the command that crumbkit hands the terminal to"
    )]
    let mut shell = std::process::Command::new("sh");
    shell.args(["-c", "exit 0"]);
    let _ = crate::child::hand_over(&mut crate::Output::default(), Spelling::COLONS, shell);
    signal_hook::low_level::raise(signal_hook::consts::SIGINT).unwrap();
    std::thread::sleep(Duration::from_millis(200));
    #[expect(
        clippy::disallowed_methods,
        reason = "the child of the owned hand-over test shows that it outlived SIGINT through its exit code"
    )]
    std::process::exit(0);
}
