//! Tests for the exit code and the note of a result whose rows report a
//! problem through the field that `Row::PROBLEM` names (ADR-0050).

use std::fmt;
use std::io;
use std::process::ExitCode;

use clap::CommandFactory as _;
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Value, json};

use super::{Failing, Tool, finish_output, finish_with};
use crate::answered::answered;
use crate::help::Guide;
use crate::{
    Batch, Code, Document, Error, Failure, Listing, Output, Recovery, Row, Sink, Spelling,
    finish_to,
};

const CHECKS: Document<Listing<Check>> = Document::new("lint", "checks", 1, "The test rows.");
const CHECK: Document<Check> = Document::new("lint", "check", 1, "The test rows.");
const CHECK_LIST: Document<Vec<Check>> = Document::new("lint", "check-list", 1, "The test rows.");
const CHECK_BATCH: Document<Batch<Check, Unreadable>> =
    Document::new("lint", "check-batch", 1, "The test rows.");

const UNREADABLE: Code = Code::new("lint", "checks", "unreadable");

/// A row whose `problem` field marks a problem when it holds a value.
#[derive(Serialize, JsonSchema)]
struct Check {
    /// The name of the check.
    name: String,
    /// What is wrong, or nothing when the check passes.
    problem: Value,
}

impl Row for Check {
    const COLUMNS: &'static [&'static str] = &["name", "problem"];
    const PROBLEM: Option<&'static str> = Some("problem");
}

/// Returns a check named `name` whose `problem` field holds `problem`.
fn check(name: &str, problem: Value) -> Check {
    Check {
        name: name.to_owned(),
        problem,
    }
}

/// Returns a check that passes.
fn passing(name: &str) -> Check {
    check(name, json!(""))
}

/// Returns a check that reports a problem.
fn broken(name: &str) -> Check {
    check(name, json!("the target is missing"))
}

/// A failure of a batch.
#[derive(Debug)]
struct Unreadable;

impl fmt::Display for Unreadable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the check cannot be read")
    }
}

impl std::error::Error for Unreadable {}

impl Failure for Unreadable {
    fn code(&self) -> &'static Code {
        &UNREADABLE
    }

    fn recovery(&self) -> Recovery {
        Recovery::Retry
    }
}

#[test]
fn a_listing_whose_rows_report_no_problem_exits_with_0() {
    let listing = Listing::new(vec![passing("one"), passing("two")], 2).described("checks");

    let finished = finish_with(&[], CHECKS, listing);

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(finished.err, "");
}

#[test]
fn a_listing_with_a_row_whose_problem_field_holds_text_exits_with_1() {
    let listing = Listing::new(vec![broken("one"), passing("two")], 2).described("checks");

    let finished = finish_with(&[], CHECKS, listing);

    assert_eq!(finished.code, ExitCode::FAILURE);
}

#[test]
fn a_problem_field_that_holds_null_false_or_an_empty_string_reports_no_problem() {
    let rows = vec![
        check("null", Value::Null),
        check("false", json!(false)),
        check("empty", json!("")),
    ];

    let finished = finish_with(&["--json"], CHECKS, Listing::new(rows, 3));

    assert_eq!(finished.code, ExitCode::SUCCESS);
}

#[test]
fn a_problem_field_that_holds_true_reports_a_problem() {
    let finished = finish_with(&["--json"], CHECK, check("flag", json!(true)));

    assert_eq!(finished.code, ExitCode::FAILURE);
}

#[test]
fn a_single_row_and_a_list_whose_rows_report_a_problem_exit_with_1() {
    let single = finish_with(&[], CHECK, broken("one"));
    let list = finish_with(&[], CHECK_LIST, vec![passing("one"), broken("two")]);

    assert_eq!(single.code, ExitCode::FAILURE);
    assert_eq!(list.code, ExitCode::FAILURE);
}

#[test]
fn a_batch_failure_decides_the_exit_before_a_row_that_reports_a_problem() {
    const MAPPED: Guide =
        Guide::new()
            .exits(&[crate::help::Exit::new(3, "A check is unreadable.").failures(&[&UNREADABLE])]);
    let mut output = guided(&MAPPED, &[]);
    let batch = answered(vec![broken("one")], vec![Unreadable]);

    let finished = finish_output(&mut output, CHECK_BATCH, batch);

    assert_eq!(finished.code, ExitCode::from(3));
    assert!(!finished.err.contains("report"), "{}", finished.err);
}

#[test]
fn a_row_that_the_output_budget_left_unprinted_still_decides_the_exit() {
    let mut rows: Vec<Check> = (0..40).map(|n| passing(&"a".repeat(n + 60))).collect();
    rows.push(broken("last"));
    let mut output = guided(
        &Guide::new().output_budget(1024, "the tests print little"),
        &[],
    );

    let finished = finish_output(&mut output, CHECKS, Listing::new(rows, 41));

    assert!(!finished.out.contains("last"), "{}", finished.out);
    assert_eq!(finished.code, ExitCode::FAILURE);
}

#[test]
fn a_listing_with_problem_rows_exits_with_1_after_the_reader_closed_the_pipe() {
    let mut err = Vec::new();
    let mut sink = Sink::new(Failing(io::ErrorKind::BrokenPipe), &mut err);
    let result: Result<_, Error> = Ok(Listing::new(vec![broken("one")], 1));

    let code = finish_to(
        &mut sink,
        &mut Output::of(clap::Command::new("tool")),
        Spelling::COLONS,
        CHECKS,
        result,
    );

    assert_eq!(code, 1);
}

#[test]
fn the_text_of_a_listing_with_problem_rows_ends_with_a_note_that_counts_them() {
    let rows = vec![broken("one"), broken("two"), passing("three")];
    let listing = Listing::new(rows, 3)
        .described("checks")
        .note("note: from the tests");

    let finished = finish_with(&[], CHECKS, listing);

    assert_eq!(
        finished.err,
        "note: 2 of the checks report a problem, so the command exits with 1.\n\
         note: from the tests\n"
    );
}

#[test]
fn the_note_of_one_problem_row_without_a_description_names_rows() {
    let finished = finish_with(&[], CHECK_LIST, vec![broken("one"), passing("two")]);

    assert_eq!(
        finished.err,
        "note: 1 of the rows reports a problem, so the command exits with 1.\n"
    );
}

#[test]
fn the_json_document_of_a_listing_with_problem_rows_has_no_new_field() {
    let listing = Listing::new(vec![broken("one")], 1);

    let finished = finish_with(&["--json"], CHECKS, listing);

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"lint/checks/v1","data":[{"name":"one","problem":"the target is missing"}],"#,
            r#""total":1,"truncated":false}"#,
            "\n"
        )
    );
    assert_eq!(finished.err, "");
}

#[test]
fn a_full_json_listing_reads_the_problem_field_although_no_column_is_selected() {
    let finished = finish_with(
        &["--json", "--full"],
        CHECKS,
        Listing::new(vec![broken("one")], 1),
    );

    assert_eq!(finished.code, ExitCode::FAILURE);
}

#[test]
fn fields_that_leave_out_the_problem_column_keep_the_exit_at_1() {
    let finished = finish_with(
        &["--fields", "name"],
        CHECKS,
        Listing::new(vec![broken("one")], 1),
    );

    assert_eq!(finished.out, "NAME\none\n");
    assert_eq!(finished.code, ExitCode::FAILURE);
}

/// Returns the output flags for `flags` with `guide`, as `Output::new`
/// builds them.
fn guided(guide: &Guide, flags: &[&str]) -> Output {
    let args: Vec<&str> = ["tool", "list"].iter().chain(flags).copied().collect();
    super::built_with(Tool::command(), guide, &args)
}
