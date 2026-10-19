//! The signal tests on Unix, where a test raises SIGTERM or SIGINT, and a
//! child that crumbkit hands the terminal to runs through `sh`.

use std::os::unix::process::ExitStatusExt as _;
use std::process::ExitCode;
use std::time::Duration;

use clap::Command;

use super::typed;
use super::{is_child, log_lines, run_child};
use crate::recorder::start;
use crate::testing::TempDir;
use crumbkit::Spelling;

#[test]
fn a_signal_writes_an_interrupted_line_and_then_ends_the_process_as_the_signal_does() {
    let state = TempDir::new("signal");

    let status = run_child(
        "recorder::tests::processes::unix::the_signal_test_s_child_starts_the_log_and_raises_sigterm",
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
fn ctrl_c_while_a_child_holds_the_terminal_ends_only_the_child_and_the_line_records_it() {
    let state = TempDir::new("hand-over");

    let status = run_child(
        "recorder::tests::processes::unix::the_hand_over_test_s_child_hands_the_terminal_to_a_child_that_ctrl_c_ends",
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
    let code = crumbkit_child::hand_over(super::bare_output(), shell);
    let number = if code == ExitCode::from(130) { 130 } else { 1 };
    #[expect(
        clippy::disallowed_methods,
        reason = "the child of the hand-over test reports the tool's exit code through its own"
    )]
    std::process::exit(number);
}

#[test]
fn a_sigint_that_arrives_after_a_child_has_held_the_terminal_leaves_the_tool_running() {
    let state = TempDir::new("late-hand-over");

    let status = run_child(
        "recorder::tests::processes::unix::the_late_hand_over_test_s_child_raises_sigint_after_its_child_exits",
        &state,
    );

    assert_eq!(status.code(), Some(0), "the tool ended with {status:?}");
    let lines = log_lines(&state, "late");
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["exit"], 0);
}

/// Runs only as the child of the late hand-over test, which sets [`CHILD`].
/// It starts the log, which listens for SIGINT, hands the terminal to a
/// shell that exits at once, which writes the line, then raises SIGINT, as
/// a Ctrl-C sent while the child ran can arrive after the child exits, and
/// ends with exit 0 if it is still running.
#[test]
fn the_late_hand_over_test_s_child_raises_sigint_after_its_child_exits() {
    if !is_child() {
        return;
    }
    let _log = start();
    typed(&Command::new("late"), &["late".into()], Spelling::COLONS);
    #[expect(
        clippy::disallowed_methods,
        reason = "the test builds the command that crumbkit hands the terminal to"
    )]
    let mut shell = std::process::Command::new("sh");
    shell.args(["-c", "exit 0"]);
    let _ = crumbkit_child::hand_over(super::bare_output(), shell);
    signal_hook::low_level::raise(signal_hook::consts::SIGINT).unwrap();
    std::thread::sleep(Duration::from_millis(200));
    #[expect(
        clippy::disallowed_methods,
        reason = "the child of the late hand-over test shows that it outlived SIGINT through its exit code"
    )]
    std::process::exit(0);
}
