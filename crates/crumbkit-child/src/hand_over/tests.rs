//! Tests for handing the terminal to a child. These tests give the response
//! functions the outcome of a child, so they run no process. The tests in
//! `process.rs` run real children through `sh`.

use std::io;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt as _;
#[cfg(windows)]
use std::os::windows::process::ExitStatusExt as _;
use std::process::{self, ExitCode, ExitStatus};

use clap::{CommandFactory as _, Parser};

use super::{hand_over_to, respond_text};
use crumbkit::help::{Exit, Guide};
use crumbkit::{Output, Sink, Spelling, ViewFlags};

use crate::testing::assert_one_word_apart;

// The process tests run their children through `sh` and send them Unix
// signals.
#[cfg(unix)]
mod process_tests;

/// A tool with the output flags and nothing else.
#[derive(Parser)]
#[command(name = "vault")]
struct Tool {
    #[command(flatten)]
    view: ViewFlags,
}

/// Parses `vault` followed by `flags` into the output of the run, as a
/// tool that parses with clap alone builds it.
fn output(flags: &[&str]) -> Output {
    let words: Vec<&str> = ["vault"].iter().chain(flags).copied().collect();
    let view = Tool::try_parse_from(&words).unwrap().view;
    Output::new(view, Tool::command(), &Guide::new(), words)
}

/// What a response returned and wrote.
struct Responded {
    code: ExitCode,
    out: String,
    err: String,
}

/// Runs `respond` with a sink over memory and returns what it wrote.
fn respond(respond: impl FnOnce(&mut Sink<&mut Vec<u8>, &mut Vec<u8>>) -> ExitCode) -> Responded {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = respond(&mut Sink::new(&mut out, &mut err));
    Responded {
        code,
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

/// Returns the status of a child that exited with `code`, as `waitpid`
/// reports it.
#[cfg(unix)]
fn exited(code: i32) -> ExitStatus {
    ExitStatus::from_raw(code << 8)
}

/// Returns the status of a child that exited with `code`, as
/// `GetExitCodeProcess` reports it.
#[cfg(windows)]
fn exited(code: i32) -> ExitStatus {
    ExitStatus::from_raw(code.cast_unsigned())
}

/// Returns the status of a child that the signal `signal` ended.
#[cfg(unix)]
fn killed(signal: i32) -> ExitStatus {
    ExitStatus::from_raw(signal)
}

/// Returns the status of a child that Ctrl-C ended, which exits with
/// `STATUS_CONTROL_C_EXIT`.
#[cfg(windows)]
fn interrupted() -> ExitStatus {
    ExitStatus::from_raw(0xC000_013A)
}

/// Returns what `respond_text` does for a child that ended with `status`.
fn text(status: io::Result<ExitStatus>) -> Responded {
    let mut output = output(&[]);
    respond(|sink| respond_text(sink, &mut output, "vi".to_owned(), status))
}

#[test]
fn a_child_that_holds_the_terminal_returns_its_own_exit_code_and_prints_nothing() {
    for code in [0, 1, 2, 3, 255] {
        let responded = text(Ok(exited(code)));

        assert_eq!(responded.code, ExitCode::from(u8::try_from(code).unwrap()));
        assert_eq!((responded.out.as_str(), responded.err.as_str()), ("", ""));
    }
}

// Only Unix ends a process with a signal number.
#[cfg(unix)]
#[test]
fn a_child_that_a_signal_ended_returns_128_plus_the_signal_number() {
    assert_eq!(text(Ok(killed(2))).code, ExitCode::from(130));
    assert_eq!(text(Ok(killed(15))).code, ExitCode::from(143));
}

// Only Unix ends a process with a signal number.
#[cfg(unix)]
#[test]
fn a_child_that_a_signal_ended_reports_that_it_was_interrupted_and_then_its_exit() {
    let seen = crate::testing::watch(|| drop(text(Ok(killed(2)))));

    assert_eq!(seen, ["interrupted", "ended 130 None"]);
}

#[test]
fn a_missing_program_fails_with_not_found_and_a_hint_to_install_it() {
    let missing = io::Error::from(io::ErrorKind::NotFound);
    let responded = text(Err(missing));

    assert_eq!(responded.code, ExitCode::FAILURE);
    assert_eq!(responded.out, "");
    let expected = "\
error: cannot run `vi`: no such program
  help:  install `vi`, or put it on PATH
  code:  vault::child::not_found
";
    assert_eq!(responded.err, expected);
}

#[test]
fn a_program_that_cannot_start_fails_with_not_started_and_its_cause() {
    let refused = io::Error::new(io::ErrorKind::PermissionDenied, "permission denied");
    let responded = text(Err(refused));

    assert_eq!(responded.code, ExitCode::FAILURE);
    let expected = "\
error: cannot run `vi`
  cause: permission denied
  help:  check that `vi` is an executable file
  code:  vault::child::not_started
";
    assert_eq!(responded.err, expected);
}

#[cfg(windows)]
#[test]
fn a_child_that_ctrl_c_ended_returns_130_as_a_child_that_sigint_ended_does() {
    assert_eq!(text(Ok(interrupted())).code, ExitCode::from(130));
}

#[cfg(windows)]
#[test]
fn a_windows_exit_code_above_255_that_ctrl_c_did_not_give_returns_1() {
    assert_eq!(text(Ok(exited(-1))).code, ExitCode::from(1));
}

/// Returns a command that runs `program`.
fn command(program: &str) -> process::Command {
    #[expect(
        clippy::disallowed_methods,
        reason = "the tests build the commands that crumbkit hands the terminal to"
    )]
    process::Command::new(program)
}

/// A program that no test machine has, so a test that ran it would fail
/// with `vault::child::not_found`.
const MISSING: &str = "crumbkit-test-no-such-program";

/// Checks the twin of a refusal that comes before the child runs: `flags`
/// differ from `refused` by one word, and with them the child runs, so the
/// missing program fails with `vault::child::not_found` (TEST-1).
#[track_caller]
fn admits(refused: &[&str], flags: &[&str]) {
    assert_one_word_apart(refused, flags);
    let mut output = output(flags);
    let responded = respond(|sink| hand_over_to(sink, &mut output, command(MISSING)));
    let printed = format!("{}{}", responded.out, responded.err);
    assert!(printed.contains("vault::child::not_found"), "{printed}");
}

#[test]
fn json_is_refused_with_the_usage_code_before_the_child_runs() {
    admits(&["--json"], &[]);
    let mut output = output(&["--json"]);
    let responded = respond(|sink| hand_over_to(sink, &mut output, command(MISSING)));

    assert_eq!(responded.code, ExitCode::from(2));
    let document: serde_json::Value = serde_json::from_str(&responded.out).unwrap();
    assert_eq!(document["code"], "vault::cli::usage");
    assert_eq!(
        document["message"],
        format!("--json is not available, because this command hands the terminal to `{MISSING}`")
    );
    assert_eq!(
        document["recovery"]["text"],
        format!("run the command without --json to read the output of `{MISSING}`")
    );
    assert_eq!(document["fields"]["argument"], "--json");
}

#[test]
fn fields_are_refused_with_the_usage_code_before_the_child_runs() {
    let mut output = output(&["--fields", "title"]);
    let responded = respond(|sink| hand_over_to(sink, &mut output, command(MISSING)));

    assert_eq!(responded.code, ExitCode::from(2));
    let expected = "\
error: --fields selects the columns of a table, and this command hands the terminal to another program
  help:  run the command without --fields
  code:  vault::cli::usage
";
    assert_eq!(responded.err, expected);
}

/// A guide that gives a refused command line the exit code 64.
const USAGE_64: Guide = Guide::new().exits(&[
    Exit::new(64, "The command line was refused.").failures(&[&crumbkit::codes::CLI_USAGE])
]);

#[test]
fn a_refused_json_exits_with_the_code_that_the_guide_gives_a_refused_command_line() {
    let words = ["vault", "--json"];
    let view = Tool::try_parse_from(words).unwrap().view;
    let mut output = Output::new(view, Tool::command(), &USAGE_64, words);

    let code = respond(|sink| hand_over_to(sink, &mut output, command(MISSING))).code;

    assert_eq!(code, ExitCode::from(64));
}

/// A spelling that joins the segments of a code with `.`.
const DOTS: Spelling =
    Spelling::custom(|code, f| write!(f, "{}.{}.{}", code.component(), code.area(), code.reason()));

/// Parses `vault` followed by `flags` into the output of the run, as a tool
/// that parses with clap alone and sets the dotted spelling on its guide
/// builds it.
fn dotted(flags: &[&str]) -> Output {
    const GUIDE: Guide = Guide::new().spelling(DOTS);
    let words: Vec<&str> = ["vault"].iter().chain(flags).copied().collect();
    let view = Tool::try_parse_from(&words).unwrap().view;
    Output::new(view, Tool::command(), &GUIDE, words)
}

#[test]
fn a_guide_with_a_custom_spelling_prints_it_through_hand_over_without_a_further_argument() {
    let mut output = dotted(&["--json"]);

    let responded = respond(|sink| hand_over_to(sink, &mut output, command("vi")));

    assert!(
        responded.out.contains(r#""code":"vault.cli.usage""#),
        "{}",
        responded.out
    );
}
