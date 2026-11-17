//! Tests for the exit code and the note of a result whose rows report a
//! warning, as `Row::WARNING` decides (ADR-0245).

use std::process::ExitCode;

use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Value, json};

use std::ops::ControlFlow;

use clap::CommandFactory as _;

use super::{Cli, Failing, finish_with, parse_against};
use crate::{
    Batch, Bytes, Document, Error, Headed, Listing, Problem, Row, Run, Sink, Stream, Warning,
    finish_to, stream_to,
};

const CHECKS: Document<Listing<Check>> = Document::new("lint", "checks", 1, "The test rows.");
const CHECK_PAGES: Document<Batch<Page, Error>> =
    Document::new("lint", "pages", 1, "The test pages.").headed();
const ONLY_WARNED: Document<Listing<Check>> =
    Document::new("lint", "checks", 1, "The test rows.").only_problems();

/// A row whose `problem` field marks a problem and whose `warning` field
/// marks a warning when each holds a value.
#[derive(Serialize, JsonSchema)]
struct Check {
    /// The name of the check.
    name: String,
    /// What is wrong, or nothing when the check passes.
    problem: Value,
    /// What deserves attention, or nothing.
    warning: Value,
}

impl Row for Check {
    const COLUMNS: &'static [&'static str] = &["name", "problem", "warning"];
    const PROBLEM: Option<Problem<Self>> = Some(Problem::field("problem"));
    const WARNING: Option<Warning<Self>> = Some(Warning::field("warning"));

    fn in_default_text(&self) -> bool {
        false
    }
}

/// Returns a check with `problem` and `warning`.
fn check(name: &str, problem: Value, warning: Value) -> Check {
    Check {
        name: name.to_owned(),
        problem,
        warning,
    }
}

/// Returns a check that passes.
fn passing(name: &str) -> Check {
    check(name, json!(""), json!(""))
}

/// Returns a check that reports a warning.
fn warned(name: &str) -> Check {
    check(name, json!(""), json!("the lock file is old"))
}

/// Returns a check that reports a problem.
fn broken(name: &str) -> Check {
    check(name, json!("the target is missing"), json!(""))
}

#[test]
fn a_listing_whose_rows_report_only_warnings_exits_with_0_and_notes_their_count() {
    let rows = vec![warned("one"), warned("two"), passing("three")];

    let finished = finish_with(&[], CHECKS, Listing::new(rows, 3).described("checks"));

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(finished.err, "note: 2 of the checks report a warning.\n");
}

#[test]
fn the_note_of_one_warning_row_without_a_description_names_rows() {
    let finished = finish_with(&[], CHECKS, Listing::new(vec![warned("one")], 1));

    assert_eq!(finished.err, "note: 1 of the rows reports a warning.\n");
}

#[test]
fn a_row_that_reports_a_problem_and_a_warning_counts_only_as_a_problem() {
    let both = check("one", json!("missing"), json!("old"));

    let finished = finish_with(&[], CHECKS, Listing::new(vec![both], 1).described("checks"));

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert_eq!(
        finished.err,
        "note: 1 of the checks reports a problem, so the command exits with 1.\n"
    );
}

#[test]
fn the_warning_note_follows_the_problem_note_and_precedes_the_tools_note() {
    let rows = vec![warned("one"), broken("two")];
    let listing = Listing::new(rows, 2)
        .described("checks")
        .note("note: from the tests");

    let finished = finish_with(&[], CHECKS, listing);

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert_eq!(
        finished.err,
        "note: 1 of the checks reports a problem, so the command exits with 1.\n\
         note: 1 of the checks reports a warning.\n\
         note: from the tests\n"
    );
}

#[test]
fn a_run_with_a_failure_prints_no_warning_note() {
    let mut batch = Batch::new(&["show"], "names", &["a", "b"]);
    batch.extend([Ok(page("a", true)), Err(Error::unclassified("no page b"))]);

    let finished = finish_with(&[], CHECK_PAGES, batch);

    assert!(finished.err.contains("no page b"), "{}", finished.err);
    assert!(!finished.err.contains("warning"), "{}", finished.err);
}

#[test]
fn the_default_text_lists_a_warning_row_that_in_default_text_leaves_out() {
    let rows = vec![passing("quiet"), warned("loud")];

    let finished = finish_with(&[], CHECKS, Listing::new(rows, 2));

    assert!(finished.out.contains("loud"), "{}", finished.out);
    assert!(!finished.out.contains("quiet"), "{}", finished.out);
}

#[test]
fn an_only_problems_listing_lists_its_warning_rows() {
    let rows = vec![broken("bad"), warned("worn"), passing("fine")];

    let finished = finish_with(&[], ONLY_WARNED, Listing::new(rows, 3));

    assert!(finished.out.contains("bad"), "{}", finished.out);
    assert!(finished.out.contains("worn"), "{}", finished.out);
    assert!(!finished.out.contains("fine"), "{}", finished.out);
}

#[test]
fn a_listing_cut_by_its_limit_counts_the_warnings_of_the_rows_it_cut() {
    let rows = vec![passing("one"), warned("two"), warned("three")];
    let listing = Listing::from_all(rows, Some(1)).described("checks");

    let finished = finish_with(&[], CHECKS, listing);

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(
        finished.err,
        "Showing 1 of 3 checks. --all shows every one.\n\
         note: 2 of the checks report a warning.\n"
    );
}

#[test]
fn a_listing_whose_rows_report_only_warnings_exits_with_0_after_the_reader_closed_the_pipe() {
    let mut err = Vec::new();
    let mut sink = Sink::new(Failing(std::io::ErrorKind::BrokenPipe), &mut err);
    let result: Result<_, Error> = Ok(Listing::new(vec![warned("one")], 1));

    let code = finish_to(
        &mut sink,
        Run::of(clap::Command::new("tool")),
        CHECKS,
        result,
    );

    assert_eq!(code, 0);
}

#[test]
fn warnings_add_no_key_to_the_json_document() {
    let finished = finish_with(&["--json"], CHECKS, Listing::new(vec![warned("one")], 1));

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"lint/checks/v1","data":[{"name":"one","problem":"","warning":"the lock file is old"}],"#,
            r#""total":1,"truncated":false}"#,
            "\n"
        )
    );
    assert_eq!(finished.err, "");
}

/// A page of a headed batch that reports a warning when it is `worn`.
#[derive(Serialize, JsonSchema)]
struct Page {
    /// The page's name.
    name: String,
    /// Whether the page is worn.
    worn: bool,
    /// The page's bytes.
    #[serde(flatten)]
    content: Option<Bytes>,
}

impl Row for Page {
    const COLUMNS: &'static [&'static str] = &["name", "worn"];
    const WARNING: Option<Warning<Self>> = Some(Warning::field("worn"));
}

impl Headed for Page {
    fn bytes(&self) -> Option<&Bytes> {
        self.content.as_ref()
    }
}

/// Returns a page `name` that is `worn` or not.
fn page(name: &str, worn: bool) -> Page {
    Page {
        name: name.to_owned(),
        worn,
        content: Some(Bytes::from(b"text\n".to_vec())),
    }
}

#[test]
fn a_headed_batch_notes_its_warning_rows() {
    let mut batch = Batch::new(&["show"], "names", &["a", "b"]);
    batch.extend([Ok(page("a", true)), Ok::<_, Error>(page("b", false))]);

    let finished = finish_with(&[], CHECK_PAGES, batch);

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(finished.err, "note: 1 of the rows reports a warning.\n");
}

#[test]
fn a_stream_whose_items_report_warnings_exits_with_0_and_notes_them() {
    let checks: Document<Stream<Check>> = Document::new("lint", "stream", 1, "Each check.");
    let output = parse_against(Cli::command(), &[]);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);
    let mut streaming = stream_to(&mut sink, output, checks).unwrap();
    for check in [passing("one"), warned("two"), warned("three")] {
        assert_eq!(streaming.item(&check), ControlFlow::Continue(()));
    }

    let code = streaming.end(Ok::<(), Error>(()));

    assert_eq!(code, ExitCode::SUCCESS);
    assert_eq!(
        String::from_utf8(err).unwrap(),
        "note: 2 of the rows report a warning.\n"
    );
}
