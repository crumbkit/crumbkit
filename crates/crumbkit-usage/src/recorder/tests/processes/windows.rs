//! The signal tests on Windows, where a test raises SIGINT, as Ctrl-C does,
//! and a child that crumbkit hands the terminal to runs through `cmd`.

use std::process::ExitCode;
use std::time::{Duration, Instant};

use clap::Command;

use super::typed;
use super::{is_child, log_lines, run_child};
use crate::recorder::{RECORDER, Signals, State, ended, start, start_with};
use crate::testing::TempDir;
use crumbkit::Spelling;

/// The exit code of a process that Ctrl-C ended, `STATUS_CONTROL_C_EXIT`.
#[expect(
    clippy::cast_possible_wrap,
    reason = "Windows exit codes are 32-bit, and this one is above `i32::MAX` by design"
)]
const CONTROL_C_EXIT: i32 = 0xC000_013A_u32 as i32;

/// Returns `cmd /C` with `script`, which crumbkit hands the terminal to.
fn cmd(script: &str) -> std::process::Command {
    #[expect(
        clippy::disallowed_methods,
        reason = "the test builds the command that crumbkit hands the terminal to"
    )]
    let mut cmd = std::process::Command::new("cmd");
    cmd.args(["/C", script]);
    cmd
}

/// Ends this child process with `code`, which the parent test reads.
fn exit(code: i32) -> ! {
    #[expect(
        clippy::disallowed_methods,
        reason = "the child of a signal test reports what happened through its exit code"
    )]
    std::process::exit(code)
}

#[test]
fn ctrl_c_writes_an_interrupted_line_and_ends_the_process_as_ctrl_c_does() {
    let state = TempDir::new("signal");

    let status = run_child(
        "recorder::tests::processes::windows::the_ctrl_c_test_s_child_starts_the_log_and_raises_sigint",
        &state,
    );

    assert_eq!(
        status.code(),
        Some(CONTROL_C_EXIT),
        "the tool ended with {status:?}"
    );
    let lines = log_lines(&state, "signalled");
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["interrupted"], true);
}

/// Runs only as the child of the Ctrl-C test, which sets the variable
/// that `is_child` reads. It starts the log, gives it the tool's name as
/// `parse` does, and raises SIGINT, which the log's thread answers.
#[test]
fn the_ctrl_c_test_s_child_starts_the_log_and_raises_sigint() {
    if !is_child() {
        return;
    }
    let _log = start();
    typed(
        &Command::new("signalled"),
        &["signalled".into()],
        Spelling::COLONS,
    );
    signal_hook::low_level::raise(signal_hook::consts::SIGINT).unwrap();
    // The signal ends the process, so the test never gets past the wait.
    std::thread::sleep(Duration::from_secs(10));
    panic!("SIGINT did not end the process");
}

#[test]
fn a_tool_that_owns_its_signals_keeps_running_after_ctrl_c_and_reports_its_own_exit() {
    let state = TempDir::new("owned");

    let status = run_child(
        "recorder::tests::processes::windows::the_owned_ctrl_c_test_s_child_raises_sigint_and_ends_with_124",
        &state,
    );

    assert!(status.success(), "the tool ended with {status:?}");
    let lines = log_lines(&state, "owner");
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["interrupted"], true);
    assert_eq!(lines[0]["exit"], 124);
}

/// Runs only as the child of the owned Ctrl-C test. It starts the log for
/// a tool that owns its signals, raises SIGINT, waits until the log's
/// thread has recorded it, and reports exit 124.
#[test]
fn the_owned_ctrl_c_test_s_child_raises_sigint_and_ends_with_124() {
    if !is_child() {
        return;
    }
    let _log = start_with(Signals::Tool);
    typed(&Command::new("owner"), &["owner".into()], Spelling::COLONS);
    signal_hook::low_level::raise(signal_hook::consts::SIGINT).unwrap();
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

#[test]
fn ctrl_c_while_a_child_holds_the_terminal_leaves_the_tool_running_and_a_later_one_ends_it() {
    let state = TempDir::new("hand-over");

    let status = run_child(
        "recorder::tests::processes::windows::the_hand_over_test_s_child_raises_sigint_during_and_after_its_child",
        &state,
    );

    assert_eq!(
        status.code(),
        Some(CONTROL_C_EXIT),
        "the tool ended with {status:?}"
    );
    let lines = log_lines(&state, "editing");
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["exit"], 0);
    assert_eq!(lines[0].get("interrupted"), None);
}

/// Runs only as the child of the hand-over test. It starts the log and
/// hands the terminal to a `cmd` that waits about two seconds, while
/// another thread raises SIGINT. After the child exits, which writes the
/// line, it raises SIGINT again, which ends the tool.
#[test]
fn the_hand_over_test_s_child_raises_sigint_during_and_after_its_child() {
    if !is_child() {
        return;
    }
    let _log = start();
    typed(
        &Command::new("editing"),
        &["editing".into()],
        Spelling::COLONS,
    );
    let raiser = std::thread::spawn(|| {
        std::thread::sleep(Duration::from_millis(500));
        signal_hook::low_level::raise(signal_hook::consts::SIGINT).unwrap();
    });
    let code =
        crumbkit_child::hand_over(&mut super::bare_output(), cmd("ping -n 3 127.0.0.1 >NUL"));
    raiser.join().unwrap();
    if code != ExitCode::SUCCESS {
        exit(1);
    }
    signal_hook::low_level::raise(signal_hook::consts::SIGINT).unwrap();
    std::thread::sleep(Duration::from_secs(10));
    exit(2);
}

#[test]
fn a_child_that_ctrl_c_ended_returns_130_and_the_line_records_an_interrupt() {
    let state = TempDir::new("hand-over-ended");

    let status = run_child(
        "recorder::tests::processes::windows::the_interrupted_child_test_s_child_hands_over_to_a_child_that_ctrl_c_ended",
        &state,
    );

    assert_eq!(status.code(), Some(130), "the tool ended with {status:?}");
    let lines = log_lines(&state, "editing");
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["interrupted"], true);
    assert_eq!(lines[0]["exit"], 130);
}

/// Runs only as the child of the interrupted-child test. It starts the log
/// and hands the terminal to a `cmd` that exits with
/// `STATUS_CONTROL_C_EXIT`, as a child that Ctrl-C ended does.
#[test]
fn the_interrupted_child_test_s_child_hands_over_to_a_child_that_ctrl_c_ended() {
    if !is_child() {
        return;
    }
    let _log = start();
    typed(
        &Command::new("editing"),
        &["editing".into()],
        Spelling::COLONS,
    );
    let code = crumbkit_child::hand_over(
        &mut super::bare_output(),
        cmd(&format!("exit {CONTROL_C_EXIT}")),
    );
    exit(if code == ExitCode::from(130) { 130 } else { 1 });
}
