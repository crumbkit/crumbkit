//! Tests for clap's usage lines, which the text form of a refused command
//! line prints under its `error:` line.

use super::super::{PreparedFailure, Printing, write_crumbkit_text};
use super::{Missing, missing, run_list, two_causes};
use crate::Spelling;

/// Prepares `failure` with the usage lines `usage`, as a refusal from clap
/// prepares itself.
fn prepared<'a>(failure: &'a Missing, usage: &'a [String]) -> PreparedFailure<'a> {
    PreparedFailure::new(
        failure,
        Spelling::COLONS,
        Printing::default(),
        &mut clap::Command::new("tool").subcommand(clap::Command::new("list")),
        &mut crate::sink::role::Recorded::default(),
    )
    .with_usage(usage)
}

/// Returns the text form of `document`.
fn text(document: &PreparedFailure<'_>) -> String {
    let mut err = Vec::new();
    write_crumbkit_text(&mut err, document).unwrap();
    String::from_utf8(err).unwrap()
}

#[test]
fn each_usage_line_prints_under_the_error_line_and_before_the_help_lines() {
    let failure = missing(run_list());
    let usage = ["tool <NAME>".to_owned(), "tool <COMMAND>".to_owned()];

    let expected = "\
error: the solar system has no planet Vulcan
  usage: tool <NAME>
  usage: tool <COMMAND>
  help:  run `tool list`
  code:  solar::planets::not_found
";
    assert_eq!(text(&prepared(&failure, &usage)), expected);
}

#[test]
fn the_usage_lines_print_before_the_cause_lines() {
    let failure = Missing {
        cause: Some(two_causes()),
        ..missing(run_list())
    };
    let usage = ["tool list".to_owned()];

    let printed = text(&prepared(&failure, &usage));

    assert!(
        printed.starts_with(
            "error: the solar system has no planet Vulcan\n  usage: tool list\n  cause: "
        ),
        "{printed}"
    );
}

#[test]
fn the_failure_document_holds_no_usage() {
    let failure = missing(run_list());
    let usage = ["tool list".to_owned()];

    let document = serde_json::to_value(prepared(&failure, &usage)).unwrap();

    assert_eq!(document.get("usage"), None);
    assert_eq!(
        document.get("message").and_then(serde_json::Value::as_str),
        Some("the solar system has no planet Vulcan")
    );
}
