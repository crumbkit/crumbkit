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
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Json, hand_over_json_to, hand_over_to, respond_json, respond_text};
use crate::twins::assert_one_word_apart;
use crate::{Document, Output, Row, Sink, Spelling};

// The process tests run their children through `sh` and send them Unix
// signals.
#[cfg(unix)]
mod process_tests;

/// A row that a child prints as JSON.
#[derive(Serialize, Deserialize, JsonSchema)]
struct Book {
    title: String,
    pages: u32,
}

impl Row for Book {
    const COLUMNS: &'static [&'static str] = &["title", "pages"];
}

const BOOKS: Document<Vec<Book>> = Document::new("vault", "books-list", 1, "The test books.");

/// A tool with the output flags and nothing else.
#[derive(Parser)]
#[command(name = "vault")]
struct Tool {
    #[command(flatten)]
    output: Output,
}

/// Parses `vault` followed by `flags` into the output flags, with the
/// tool's command set as its `main` sets it.
fn output(flags: &[&str]) -> Output {
    let mut output = Tool::try_parse_from(["vault"].iter().chain(flags))
        .unwrap()
        .output;
    output.set_command(Tool::command());
    output
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
    respond(|sink| respond_text(sink, &mut output, Spelling::COLONS, "vi".to_owned(), status))
}

/// Returns what `respond_json` does under `--json` and `flags` for a child
/// that printed `stdout` and ended with `status`.
fn json(flags: &[&str], status: ExitStatus, stdout: &str) -> Responded {
    let mut output = output(&[&["--json"], flags].concat());
    let captured = process::Output {
        status,
        stdout: stdout.as_bytes().to_vec(),
        stderr: Vec::new(),
    };
    respond(|sink| {
        respond_json(
            sink,
            &mut output,
            Spelling::COLONS,
            BOOKS,
            "vault-log".to_owned(),
            Ok(captured),
        )
    })
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
    let seen = crate::events::testing::watch(|| drop(text(Ok(killed(2)))));

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
  code:  crumbkit::child::not_found
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
  code:  crumbkit::child::not_started
";
    assert_eq!(responded.err, expected);
}

#[test]
fn a_childs_json_prints_inside_the_envelope_of_the_declared_document() {
    let stdout = r#"[{"title":"Dune","pages":412}]"#;
    let responded = json(&[], exited(0), stdout);

    assert_eq!(responded.code, ExitCode::SUCCESS);
    let expected =
        "{\"schema\":\"vault/books-list/v1\",\"data\":[{\"title\":\"Dune\",\"pages\":412}]}\n";
    assert_eq!(responded.out, expected);
    assert_eq!(responded.err, "");
}

#[test]
fn fields_select_the_fields_of_a_childs_json() {
    let stdout = r#"[{"title":"Dune","pages":412}]"#;
    let responded = json(&["--fields", "pages"], exited(0), stdout);

    let expected = "{\"schema\":\"vault/books-list/v1\",\"data\":[{\"pages\":412}]}\n";
    assert_eq!(responded.out, expected);
}

#[test]
fn a_childs_output_that_is_not_the_document_fails_with_bad_json() {
    let responded = json(&[], exited(0), "Dune, 412 pages\n");

    assert_eq!(responded.code, ExitCode::FAILURE);
    let document: serde_json::Value = serde_json::from_str(&responded.out).unwrap();
    assert_eq!(document["code"], "crumbkit::child::bad_json");
    assert_eq!(
        document["message"],
        "`vault-log` printed output that is not a vault/books-list/v1 document"
    );
    assert_eq!(document["recovery"]["kind"], "none");
}

#[test]
fn a_child_that_fails_under_json_prints_failed_and_returns_its_own_code() {
    let responded = json(&[], exited(3), r#"[{"title":"Dune","pages":412}]"#);

    assert_eq!(responded.code, ExitCode::from(3));
    let document: serde_json::Value = serde_json::from_str(&responded.out).unwrap();
    assert_eq!(document["code"], "crumbkit::child::failed");
    assert_eq!(document["message"], "`vault-log` exited with code 3");
}

// Only Unix ends a process with a signal number.
#[cfg(unix)]
#[test]
fn a_child_that_a_signal_ended_under_json_returns_128_plus_the_signal_number() {
    assert_eq!(json(&[], killed(2), "").code, ExitCode::from(130));
}

#[cfg(windows)]
#[test]
fn a_child_that_ctrl_c_ended_returns_130_as_a_child_that_sigint_ended_does() {
    assert_eq!(text(Ok(interrupted())).code, ExitCode::from(130));
    assert_eq!(json(&[], interrupted(), "").code, ExitCode::from(130));
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
/// with `crumbkit::child::not_found`.
const MISSING: &str = "crumbkit-test-no-such-program";

/// Checks the twin of a refusal that comes before the child runs: `flags`
/// differ from `refused` by one word, and with them the child runs, so the
/// missing program fails with `crumbkit::child::not_found` (TEST-1).
#[track_caller]
fn admits(refused: &[&str], flags: &[&str], json: bool) {
    assert_one_word_apart(refused, flags);
    let mut output = output(flags);
    let responded = respond(|sink| {
        let child = command(MISSING);
        if json {
            hand_over_json_to(sink, &mut output, Spelling::COLONS, child, BOOKS_JSON)
        } else {
            hand_over_to(sink, &mut output, Spelling::COLONS, child)
        }
    });
    let printed = format!("{}{}", responded.out, responded.err);
    assert!(printed.contains("crumbkit::child::not_found"), "{printed}");
}

/// The JSON form of a child that prints [`BOOKS`].
const BOOKS_JSON: Json<Vec<Book>> = Json::new(&["--json"], BOOKS);

#[test]
fn json_for_a_child_without_a_json_form_is_refused_with_no_json_before_it_runs() {
    admits(&["--json"], &[], false);
    let mut output = output(&["--json"]);
    let responded = respond(|sink| {
        let child = command(MISSING);
        hand_over_to(sink, &mut output, Spelling::COLONS, child)
    });

    assert_eq!(responded.code, ExitCode::from(2));
    let document: serde_json::Value = serde_json::from_str(&responded.out).unwrap();
    assert_eq!(document["code"], "crumbkit::child::no_json");
    assert_eq!(
        document["recovery"]["text"],
        format!("run the command without --json to read the output of `{MISSING}`")
    );
}

#[test]
fn fields_without_json_are_refused_with_exit_code_2_before_the_child_runs() {
    admits(
        &["--fields", "title"],
        &["--json", "--fields", "title"],
        true,
    );
    let mut output = output(&["--fields", "title"]);
    let responded = respond(|sink| {
        let child = command(MISSING);
        hand_over_json_to(sink, &mut output, Spelling::COLONS, child, BOOKS_JSON)
    });

    assert_eq!(responded.code, ExitCode::from(2));
    let expected = "\
error: --fields selects the columns of a table, and this command hands the terminal to another program
  help:  run the command without --fields
  code:  crumbkit::cli::usage
";
    assert_eq!(responded.err, expected);
}

#[test]
fn an_unknown_field_under_json_is_refused_before_the_child_runs() {
    admits(
        &["--json", "--fields", "author"],
        &["--json", "--fields", "title"],
        true,
    );
    let mut output = output(&["--json", "--fields", "author"]);
    let responded = respond(|sink| {
        let child = command(MISSING);
        hand_over_json_to(sink, &mut output, Spelling::COLONS, child, BOOKS_JSON)
    });

    assert_eq!(responded.code, ExitCode::from(2));
    let document: serde_json::Value = serde_json::from_str(&responded.out).unwrap();
    assert_eq!(document["code"], "crumbkit::cli::usage");
}
