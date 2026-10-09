//! Tests for how `finish_to` prints a failure.

use std::fmt;
use std::io::{self, Write};
use std::process::ExitCode;

use clap::{CommandFactory as _, Parser, Subcommand};
use schemars::JsonSchema;
use serde::Serialize;

use super::FailureDocument;
use crate::{
    Call, Code, Document, Error, Failure, Output, Recovery, Row, Sink, Spelling, finish_to,
};

const NOT_FOUND: Code = Code::new("solar", "planets", "not_found");

/// A row type for the result that the failing command would have printed.
#[derive(Serialize, JsonSchema)]
struct Planet {
    name: &'static str,
}

impl Row for Planet {
    const COLUMNS: &'static [&'static str] = &["name"];
}

const PLANETS: Document<Vec<Planet>> = Document::new("solar", "planets", 1);

/// A tool with a `list` command that a recovery can name.
#[derive(Parser)]
#[command(name = "tool")]
struct Tool {
    #[command(subcommand)]
    command: Command,
    #[command(flatten)]
    output: Output,
}

#[derive(Subcommand)]
enum Command {
    List,
}

/// An error in a failure's `source()` chain, with the next error below it.
#[derive(Debug)]
struct Cause {
    message: &'static str,
    source: Option<Box<Cause>>,
}

impl fmt::Display for Cause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message)
    }
}

impl std::error::Error for Cause {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_deref()
            .map(|cause| cause as &(dyn std::error::Error + 'static))
    }
}

/// A failure whose every part a test chooses.
#[derive(Debug)]
struct Missing {
    message: &'static str,
    cause: Option<Cause>,
    recovery: Recovery,
    url: Option<&'static str>,
}

impl fmt::Display for Missing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message)
    }
}

impl std::error::Error for Missing {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.cause
            .as_ref()
            .map(|cause| cause as &(dyn std::error::Error + 'static))
    }
}

impl Failure for Missing {
    fn code(&self) -> &'static Code {
        &NOT_FOUND
    }

    fn recovery(&self) -> Recovery {
        self.recovery.clone()
    }

    fn url(&self) -> Option<&str> {
        self.url
    }
}

/// Returns a failure with no cause and no url, whose recovery is `recovery`.
fn missing(recovery: Recovery) -> Missing {
    Missing {
        message: "the solar system has no planet Vulcan",
        cause: None,
        recovery,
        url: None,
    }
}

/// Returns the recovery that names `tool list`.
fn run_list() -> Recovery {
    Recovery::Run(Call::new(&["list"]))
}

/// Returns two causes, the second below the first.
fn two_causes() -> Cause {
    Cause {
        message: "the catalogue cannot be read",
        source: Some(Box::new(Cause {
            message: "permission denied",
            source: None,
        })),
    }
}

/// What `finish_to` returned and wrote.
struct Finished {
    code: ExitCode,
    out: String,
    err: String,
}

/// Parses `tool list` followed by `flags`, sets the tool's command as its
/// `main` would, and finishes `failure` into memory.
fn fail_with(flags: &[&str], failure: impl Into<Error>) -> Finished {
    let args = ["tool", "list"].iter().chain(flags);
    let mut output = Tool::try_parse_from(args).unwrap().output;
    output.set_command(Tool::command());
    fail_output(&output, failure.into())
}

/// Finishes `result` into memory with the flags and command in `output`.
fn fail_output<E: crate::AsFailure>(output: &Output, error: E) -> Finished {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result: Result<Vec<Planet>, E> = Err(error);
    let code = finish_to(
        &mut Sink::new(&mut out, &mut err),
        output,
        Spelling::COLONS,
        PLANETS,
        result,
    );
    Finished {
        code,
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

#[test]
fn a_failure_prints_its_message_help_and_code_on_standard_error_and_exits_with_code_1() {
    let finished = fail_with(&[], missing(run_list()));

    let expected = "\
error: the solar system has no planet Vulcan
  help:  run `tool list`
  code:  solar::planets::not_found
";
    assert_eq!(finished.code, ExitCode::FAILURE);
    assert_eq!(finished.out, "");
    assert_eq!(finished.err, expected);
}

#[test]
fn each_error_in_the_source_chain_prints_as_one_cause_line_in_order() {
    let failure = Missing {
        cause: Some(two_causes()),
        ..missing(run_list())
    };

    let finished = fail_with(&[], failure);

    let expected = "\
error: the solar system has no planet Vulcan
  cause: the catalogue cannot be read
  cause: permission denied
  help:  run `tool list`
  code:  solar::planets::not_found
";
    assert_eq!(finished.err, expected);
}

#[test]
fn retry_prints_as_run_the_same_command_again_later() {
    let finished = fail_with(&[], missing(Recovery::Retry));

    assert!(
        finished
            .err
            .contains("\n  help:  run the same command again later\n"),
        "{}",
        finished.err
    );
}

#[test]
fn a_hint_prints_its_text() {
    let finished = fail_with(
        &[],
        missing(Recovery::Hint("check the spelling".to_owned())),
    );

    assert!(
        finished.err.contains("\n  help:  check the spelling\n"),
        "{}",
        finished.err
    );
}

#[test]
fn none_prints_no_next_step_and_its_reason() {
    let recovery = Recovery::None {
        reason: "the planet was never there".to_owned(),
    };

    let finished = fail_with(&[], missing(recovery));

    assert!(
        finished
            .err
            .contains("\n  help:  no next step: the planet was never there\n"),
        "{}",
        finished.err
    );
}

#[test]
fn a_url_prints_after_the_code() {
    let failure = Missing {
        url: Some("https://example.com/vulcan"),
        ..missing(Recovery::Retry)
    };

    let finished = fail_with(&[], failure);

    assert!(
        finished
            .err
            .ends_with("  code:  solar::planets::not_found\n  url:   https://example.com/vulcan\n"),
        "{}",
        finished.err
    );
}

#[test]
fn a_control_character_in_a_value_prints_as_its_escape() {
    let failure = Missing {
        message: "no planet\nVulcan",
        cause: Some(Cause {
            message: "tab\there",
            source: None,
        }),
        ..missing(Recovery::Hint("\u{1b}[1mlook\u{1b}[0m".to_owned()))
    };

    let finished = fail_with(&[], failure);

    let expected = "\
error: no planet\\nVulcan
  cause: tab\\there
  help:  \\u{1b}[1mlook\\u{1b}[0m
  code:  solar::planets::not_found
";
    assert_eq!(finished.err, expected);
}

#[test]
fn a_run_whose_call_does_not_resolve_prints_as_no_next_step() {
    let failure = missing(Recovery::Run(Call::new(&["remove"])));

    let text = fail_with(&[], missing(Recovery::Run(Call::new(&["remove"]))));
    let json = fail_with(&["--json"], failure);

    assert_eq!(text.code, ExitCode::FAILURE);
    assert!(
        text.err
            .contains("\n  help:  no next step: the suggested command is not available\n"),
        "{}",
        text.err
    );
    assert!(
        json.out.contains(
            r#""recovery":{"kind":"none","reason":"the suggested command is not available"}"#
        ),
        "{}",
        json.out
    );
}

#[test]
fn a_run_prints_as_no_next_step_when_no_command_is_set() {
    let output = Tool::try_parse_from(["tool", "list"]).unwrap().output;

    let finished = fail_output(&output, missing(run_list()));

    assert!(
        finished
            .err
            .contains("\n  help:  no next step: the suggested command is not available\n"),
        "{}",
        finished.err
    );
}

#[test]
fn json_prints_one_failure_document_on_standard_output_and_exits_with_code_1() {
    let finished = fail_with(&["--json"], missing(run_list()));

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"crumbkit/failure/v1","code":"solar::planets::not_found","message":"the solar system has no planet Vulcan","recovery":{"kind":"run","call":"tool list"}}"#,
            "\n"
        )
    );
    assert_eq!(finished.err, "");
}

#[test]
fn json_tags_each_recovery_by_kind() {
    let hint = Recovery::Hint("check the spelling".to_owned());
    let none = Recovery::None {
        reason: "the planet was never there".to_owned(),
    };

    let retry = fail_with(&["--json"], missing(Recovery::Retry));
    let hint = fail_with(&["--json"], missing(hint));
    let none = fail_with(&["--json"], missing(none));

    assert!(
        retry.out.contains(r#""recovery":{"kind":"retry"}}"#),
        "{}",
        retry.out
    );
    assert!(
        hint.out
            .contains(r#""recovery":{"kind":"hint","text":"check the spelling"}}"#),
        "{}",
        hint.out
    );
    assert!(
        none.out
            .contains(r#""recovery":{"kind":"none","reason":"the planet was never there"}}"#),
        "{}",
        none.out
    );
}

#[test]
fn json_holds_the_causes_joined_with_a_colon_and_the_url_last() {
    let failure = Missing {
        cause: Some(two_causes()),
        url: Some("https://example.com/vulcan"),
        ..missing(Recovery::Retry)
    };

    let finished = fail_with(&["--json"], failure);

    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"crumbkit/failure/v1","code":"solar::planets::not_found","message":"the solar system has no planet Vulcan","cause":"the catalogue cannot be read: permission denied","recovery":{"kind":"retry"},"url":"https://example.com/vulcan"}"#,
            "\n"
        )
    );
}

#[test]
fn json_leaves_out_the_cause_and_the_url_when_the_failure_has_neither() {
    let finished = fail_with(&["--json"], missing(Recovery::Retry));

    assert!(!finished.out.contains(r#""cause""#), "{}", finished.out);
    assert!(!finished.out.contains(r#""url""#), "{}", finished.out);
}

#[test]
fn full_and_fields_leave_the_failure_document_unchanged() {
    let plain = fail_with(&["--json"], missing(run_list()));
    let full = fail_with(&["--json", "--full"], missing(run_list()));
    let fields = fail_with(&["--json", "--fields", "name"], missing(run_list()));
    let long = fail_with(&["--json", "--long"], missing(run_list()));

    assert_eq!(full.out, plain.out);
    assert_eq!(fields.out, plain.out);
    assert_eq!(long.out, plain.out);
}

#[test]
fn an_unknown_field_exits_with_code_2_before_the_failure_prints() {
    let finished = fail_with(&["--fields", "radius"], missing(run_list()));

    assert_eq!(finished.code, ExitCode::from(2));
    assert!(
        finished
            .err
            .starts_with("error: --fields names an unknown field 'radius'")
    );
    assert!(!finished.err.contains("Vulcan"));
}

#[test]
fn finish_prints_a_crumbkit_error_as_it_prints_the_failure_inside() {
    let output = {
        let mut output = Tool::try_parse_from(["tool", "list"]).unwrap().output;
        output.set_command(Tool::command());
        output
    };

    let direct = fail_output(&output, missing(run_list()));
    let wrapped = fail_output(&output, Error::from(missing(run_list())));

    assert_eq!(wrapped.code, ExitCode::FAILURE);
    assert_eq!(wrapped.err, direct.err);
    assert_eq!(wrapped.out, direct.out);
}

/// A writer whose every write fails with an error of `kind`.
struct Failing(io::ErrorKind);

impl Write for Failing {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(self.0, "the writer failed"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Returns the output flags for `--json` with no command set.
fn json() -> Output {
    Tool::try_parse_from(["tool", "list", "--json"])
        .unwrap()
        .output
}

/// Finishes a failure into `sink` with `output`.
fn fail_into<O: Write, W: Write>(sink: &mut Sink<O, W>, output: &Output) -> ExitCode {
    let result: Result<Vec<Planet>, Missing> = Err(missing(Recovery::Retry));
    finish_to(sink, output, Spelling::COLONS, PLANETS, result)
}

#[test]
fn a_failed_write_of_the_failure_document_exits_with_code_1_and_reports_the_cause() {
    let mut err = Vec::new();
    let mut sink = Sink::new(Failing(io::ErrorKind::Other), &mut err);

    let code = fail_into(&mut sink, &json());

    assert_eq!(code, ExitCode::FAILURE);
    assert_eq!(
        String::from_utf8(err).unwrap(),
        "error: cannot write to standard output: the writer failed\n"
    );
}

#[test]
fn a_reader_that_closes_the_pipe_early_on_a_failure_document_leaves_code_1_and_no_report() {
    let mut err = Vec::new();
    let mut sink = Sink::new(Failing(io::ErrorKind::BrokenPipe), &mut err);

    let code = fail_into(&mut sink, &json());

    assert_eq!(code, ExitCode::FAILURE);
    assert_eq!(String::from_utf8(err).unwrap(), "");
}

#[test]
fn a_failed_write_of_the_text_failure_still_exits_with_code_1() {
    let mut out = Vec::new();
    let mut sink = Sink::new(&mut out, Failing(io::ErrorKind::Other));

    let code = fail_into(&mut sink, &Output::default());

    assert_eq!(code, ExitCode::FAILURE);
    assert_eq!(String::from_utf8(out).unwrap(), "");
}

#[test]
fn the_checked_in_schema_matches_the_failure_document_schema() {
    let generated = schemars::schema_for!(FailureDocument);
    let generated = serde_json::to_string_pretty(&generated).unwrap() + "\n";
    let checked_in = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/schemas/failure-v1.json"
    ));

    assert_eq!(
        checked_in, generated,
        "schemas/failure-v1.json differs from the generated schema; replace the file with the generated schema"
    );
}
