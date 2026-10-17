//! Tests that hand the terminal to real children. Each child is a POSIX
//! `sh` script, so the tests run on every Unix machine without a test
//! program of their own.
//!
//! The signal tests run this test binary again as the tool, in a process
//! group of its own, so the SIGINT that a child sends to its process group
//! reaches the tool and the child and never the test runner.

use std::os::unix::process::CommandExt as _;
use std::process::{self, ExitCode, ExitStatus};

use super::{BOOKS_JSON, MISSING, command, output, respond};
use crumbkit::Spelling;

use crate::hand_over::{hand_over_json_to, hand_over_to};

/// The variable that tells a signal test's process to act as the tool.
const TOOL: &str = "CRUMBKIT_HAND_OVER_TOOL";

/// Returns a command that runs `script` with `sh`.
fn sh(script: &str) -> process::Command {
    let mut sh = command("sh");
    sh.args(["-c", script]);
    sh
}

/// Reports whether this process runs as the tool of a signal test.
fn is_tool() -> bool {
    #[expect(
        clippy::disallowed_methods,
        reason = "a signal test tells its tool process to run through the environment"
    )]
    std::env::var_os(TOOL).is_some()
}

/// Ends the tool process of a signal test with `code`.
fn exit_tool(code: ExitCode) -> ! {
    let number = if code == ExitCode::from(7) { 7 } else { 1 };
    #[expect(
        clippy::disallowed_methods,
        reason = "the tool process of a signal test reports the child's exit code through its own"
    )]
    process::exit(number)
}

/// Runs the test named `test` of this test binary as the tool, in a new
/// process group, and returns how it ended.
fn run_tool(test: &str) -> ExitStatus {
    let mut tool = command(std::env::current_exe().unwrap().to_str().unwrap());
    tool.args(["--exact", test, "--nocapture"])
        .env(TOOL, "1")
        .process_group(0)
        .status()
        .unwrap()
}

#[test]
fn a_real_child_gets_the_terminal_and_its_exit_code_passes_through() {
    let mut output = output(&[]);
    let responded = respond(|sink| hand_over_to(sink, &mut output, Spelling::COLONS, sh("exit 3")));

    assert_eq!(responded.code, ExitCode::from(3));
    assert_eq!((responded.out.as_str(), responded.err.as_str()), ("", ""));
}

#[test]
fn a_real_child_that_sigterm_ends_returns_143() {
    let mut output = output(&[]);
    let child = sh("kill -TERM $$");
    let responded = respond(|sink| hand_over_to(sink, &mut output, Spelling::COLONS, child));

    assert_eq!(responded.code, ExitCode::from(143));
}

#[test]
fn a_real_child_asked_for_json_with_its_declared_arguments_prints_inside_the_envelope() {
    let mut output = output(&["--json"]);
    // The script prints a book only when its last argument is `--json`,
    // which `BOOKS_JSON` adds.
    let child = sh(r#"test "$0" = --json && printf '[{"title":"Dune","pages":412}]'"#);
    let responded =
        respond(|sink| hand_over_json_to(sink, &mut output, Spelling::COLONS, child, BOOKS_JSON));

    assert_eq!(responded.code, ExitCode::SUCCESS);
    let expected =
        "{\"schema\":\"vault/books-list/v1\",\"data\":[{\"title\":\"Dune\",\"pages\":412}]}\n";
    assert_eq!(responded.out, expected);
}

#[test]
fn a_real_missing_program_fails_with_not_found() {
    let mut output = output(&[]);
    let child = command(MISSING);
    let responded = respond(|sink| hand_over_to(sink, &mut output, Spelling::COLONS, child));

    assert_eq!(responded.code, ExitCode::FAILURE);
    assert!(responded.err.contains("  code:  vault::child::not_found\n"));
}

#[test]
fn ctrl_c_reaches_only_the_child_and_the_tool_returns_the_childs_code() {
    if is_tool() {
        let mut output = output(&[]);
        // `kill -INT 0` sends SIGINT to the whole process group, as Ctrl-C
        // does: to this tool and to the child, which exits with 7.
        let child = sh("trap 'exit 7' INT; kill -INT 0; sleep 5; exit 1");
        let mut out = Vec::new();
        let mut err = Vec::new();
        let sink = &mut crumbkit::Sink::new(&mut out, &mut err);
        exit_tool(hand_over_to(sink, &mut output, Spelling::COLONS, child));
    }
    let status = run_tool(
        "hand_over::tests::process_tests::ctrl_c_reaches_only_the_child_and_the_tool_returns_the_childs_code",
    );

    assert_eq!(status.code(), Some(7), "the tool ended with {status:?}");
}

#[test]
fn sigint_and_sigquit_that_arrive_after_the_child_exits_leave_the_tool_running() {
    if is_tool() {
        let mut output = output(&[]);
        let mut out = Vec::new();
        let mut err = Vec::new();
        let sink = &mut crumbkit::Sink::new(&mut out, &mut err);
        let _ = hand_over_to(sink, &mut output, Spelling::COLONS, sh("exit 0"));
        // A Ctrl-C sent while the child ran can reach a thread of the tool
        // only after the child has exited, as these signals do.
        let _ = signal_hook::low_level::raise(signal_hook::consts::SIGINT);
        let _ = signal_hook::low_level::raise(signal_hook::consts::SIGQUIT);
        exit_tool(ExitCode::from(7));
    }
    let status = run_tool(
        "hand_over::tests::process_tests::sigint_and_sigquit_that_arrive_after_the_child_exits_leave_the_tool_running",
    );

    assert_eq!(status.code(), Some(7), "the tool ended with {status:?}");
}

#[test]
fn hand_over_reports_that_a_child_took_the_terminal_and_then_reports_its_exit() {
    let mut output = output(&[]);

    let seen = crate::testing::watch(|| {
        let _ = respond(|sink| hand_over_to(sink, &mut output, Spelling::COLONS, sh("exit 3")));
    });

    assert_eq!(seen, ["handed over", "ended 3 None"]);
}
