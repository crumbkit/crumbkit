//! Tests for the `Output` methods that a crate which prints a response
//! without `finish` uses to print failures and refuse view flags as
//! `finish` does.

use std::fmt;
use std::process::ExitCode;

use clap::{CommandFactory as _, Parser};

use super::PLANETS;
use crate::tool::{Exit, Failures};
use crate::twins::assert_one_word_apart;
use crate::{Code, Failure, Output, Recovery, Sink, Tool};

const TIMED_OUT: Code = Code::new("solar", "charts", "timed_out");
const NOT_FOUND: Code = Code::new("solar", "planets", "not_found");

/// A tool record that maps `TIMED_OUT` to 124 and leaves `NOT_FOUND` unmapped.
const TOOL: Tool = Tool::new().failures(
    Failures::new()
        .exits(&[Exit::new(124, "The star chart ran out of time.").failures(&[&TIMED_OUT])]),
);

/// A failure with the code it was given.
#[derive(Debug)]
struct Failed(&'static Code);

impl fmt::Display for Failed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the star chart failed")
    }
}

impl std::error::Error for Failed {}

impl Failure for Failed {
    fn code(&self) -> &'static Code {
        self.0
    }

    fn recovery(&self) -> Recovery {
        Recovery::Retry
    }
}

/// A tool with the output flags and nothing else.
#[derive(Parser)]
#[command(name = "tool")]
struct Cli {
    #[command(flatten)]
    view: crate::ViewFlags,
}

/// Parses `tool` followed by `flags`, and builds the output with `TOOL`,
/// as a tool that parses with clap alone builds it.
fn output_of(flags: &[&str]) -> Output {
    let view = Cli::try_parse_from(["tool"].iter().chain(flags))
        .unwrap()
        .view;
    Output::new(
        view,
        crate::DryRun::default(),
        Cli::command(),
        &TOOL,
        ["tool"].iter().chain(flags),
    )
    .unwrap()
}

/// What a method returned and wrote.
struct Printed<R> {
    returned: R,
    out: String,
    err: String,
}

/// Runs `print` with a sink over memory and returns what it wrote.
fn printed<R>(print: impl FnOnce(&mut Sink<&mut Vec<u8>, &mut Vec<u8>>) -> R) -> Printed<R> {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let returned = print(&mut Sink::new(&mut out, &mut err));
    Printed {
        returned,
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

#[test]
fn listed_exit_gives_the_exit_that_the_tool_lists_and_none_for_an_unlisted_code() {
    let output = output_of(&[]);
    assert_eq!(output.listed_exit(&TIMED_OUT), Some(124));
    assert_eq!(output.listed_exit(&NOT_FOUND), None);
}

#[test]
fn failure_exit_gives_the_listed_exit_or_crumbkit_s_default() {
    let output = output_of(&[]);
    assert_eq!(output.failure_exit(&TIMED_OUT), 124);
    assert_eq!(output.failure_exit(&NOT_FOUND), 1);
    assert_eq!(output.failure_exit(&crate::codes::CLI_USAGE), 2);
}

#[test]
fn print_failure_prints_a_failure_as_finish_prints_it() {
    let mut output = output_of(&[]);
    let printed = printed(|sink| {
        output.print_failure(sink, &Failed(&NOT_FOUND));
    });
    assert_eq!(printed.out, "");
    assert_eq!(
        printed.err,
        "error: the star chart failed\n  help:  run the same command again later\n  code:  solar::planets::not_found\n"
    );
}

#[test]
fn refuse_view_refuses_a_field_that_the_document_lacks_with_the_refusal_s_exit() {
    assert_one_word_apart(
        &["--json", "--fields", "rings"],
        &["--json", "--fields", "name"],
    );
    let mut admitted = output_of(&["--json", "--fields", "name"]);
    let admitted = printed(|sink| admitted.refuse_view(sink, PLANETS));
    assert_eq!(admitted.returned, None);
    assert_eq!(format!("{}{}", admitted.out, admitted.err), "");

    let mut refused = output_of(&["--json", "--fields", "rings"]);
    let refused = printed(|sink| refused.refuse_view(sink, PLANETS));
    assert_eq!(refused.returned, Some(ExitCode::from(2)));
    let document: serde_json::Value = serde_json::from_str(&refused.out).unwrap();
    assert_eq!(document["code"], "tool::cli::usage");
}

#[test]
fn refuse_fields_for_text_refuses_fields_with_the_caller_s_reason() {
    assert_one_word_apart(&["--fields=name"], &["--long"]);
    let mut admitted = output_of(&["--long"]);
    let admitted = printed(|sink| admitted.refuse_fields_for_text(sink, "it prints prose"));
    assert_eq!(admitted.returned, None);

    let mut refused = output_of(&["--fields=name"]);
    let refused = printed(|sink| refused.refuse_fields_for_text(sink, "it prints prose"));
    assert_eq!(refused.returned, Some(ExitCode::from(2)));
    let expected = "\
error: --fields selects the columns of a table, and it prints prose
  help:  run the command without --fields
  code:  tool::cli::usage
";
    assert_eq!(refused.err, expected);
}
