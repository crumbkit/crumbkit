//! The tests of a terminal hand-over beside crumbkit-process's forwarder.
//! Each test runs this test binary again as the tool, in a process group of
//! its own, so a signal that the tool sends to its group never reaches the
//! test runner.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, ExitCode};
use std::time::Duration;

use crumbkit::Spelling;
use crumbkit_process::forward;

use super::{bare_output, is_child, log_lines, run_child, typed};
use crate::recorder::start_under;
use crate::testing::TempDir;

/// Returns the state directory that the parent test gave this tool.
fn state_home() -> PathBuf {
    #[expect(
        clippy::disallowed_methods,
        reason = "the tool finds the directory that its test named through the environment"
    )]
    std::env::var_os("XDG_STATE_HOME").unwrap().into()
}

/// Returns a command that runs `script` with `sh`.
fn sh(script: &str) -> Command {
    #[expect(
        clippy::disallowed_methods,
        reason = "the test builds the command that crumbkit hands the terminal to"
    )]
    let mut sh = Command::new("sh");
    sh.args(["-c", script]);
    sh
}

/// The tool's clean-up, which leaves a marker for the test to find.
fn mark() {
    let state = state_home();
    #[expect(
        clippy::disallowed_methods,
        reason = "the tool's clean-up writes the marker that the test checks"
    )]
    fs::create_dir_all(&state).unwrap();
    #[expect(
        clippy::disallowed_methods,
        reason = "the tool's clean-up writes the marker that the test checks"
    )]
    fs::write(state.join("marker"), b"cleaned up\n").unwrap();
}

/// Ends this tool with `code`, which the parent test reads.
fn exit(code: i32) -> ! {
    #[expect(
        clippy::disallowed_methods,
        reason = "the tool reports what happened through its exit code"
    )]
    std::process::exit(code)
}

#[test]
fn ctrl_c_after_a_hand_over_ends_only_the_child() {
    if is_child() {
        forward::clean_up(mark);
        forward::install().unwrap();
        forward::handed_over();
        let code = crumbkit_child::hand_over(bare_output(), sh("kill -INT 0; sleep 5"));
        exit(if code == ExitCode::from(130) { 130 } else { 1 });
    }
    let state = TempDir::new("forward-hand-over");

    let status = run_child(
        "recorder::tests::processes::forward_hand_over::ctrl_c_after_a_hand_over_ends_only_the_child",
        &state,
    );

    assert_eq!(status.code(), Some(130), "the tool ended with {status:?}");
    assert!(
        !state.path().join("marker").exists(),
        "the forwarder ran the clean-up"
    );
}

#[test]
fn a_late_sigint_after_a_hand_over_leaves_the_tool_running() {
    if is_child() {
        forward::clean_up(mark);
        forward::install().unwrap();
        forward::handed_over();
        let _ = crumbkit_child::hand_over(bare_output(), sh("exit 0"));
        signal_hook::low_level::raise(signal_hook::consts::SIGINT).unwrap();
        std::thread::sleep(Duration::from_millis(200));
        exit(0);
    }
    let state = TempDir::new("forward-late");

    let status = run_child(
        "recorder::tests::processes::forward_hand_over::a_late_sigint_after_a_hand_over_leaves_the_tool_running",
        &state,
    );

    assert_eq!(status.code(), Some(0), "the tool ended with {status:?}");
    assert!(
        !state.path().join("marker").exists(),
        "the forwarder ran the clean-up"
    );
}

#[test]
fn ctrl_c_after_a_hand_over_beside_the_log_and_the_forwarder_ends_only_the_child_and_writes_the_line()
 {
    if is_child() {
        let _log = start_under(forward::clean_up);
        forward::install().unwrap();
        typed(
            &clap::Command::new("combined"),
            &["combined".into()],
            Spelling::COLONS,
        );
        forward::handed_over();
        let code = crumbkit_child::hand_over(bare_output(), sh("kill -INT 0; sleep 5"));
        exit(if code == ExitCode::from(130) { 130 } else { 1 });
    }
    let state = TempDir::new("forward-combined");

    let status = run_child(
        "recorder::tests::processes::forward_hand_over::ctrl_c_after_a_hand_over_beside_the_log_and_the_forwarder_ends_only_the_child_and_writes_the_line",
        &state,
    );

    assert_eq!(status.code(), Some(130), "the tool ended with {status:?}");
    let lines = log_lines(&state, "combined");
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["interrupted"], true);
    assert_eq!(lines[0]["exit"], 130);
}
