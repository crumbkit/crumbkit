//! Tests for the exit code and the note of a stream whose items report a
//! problem through the field that `Row::PROBLEM` names (ADR-0041,
//! ADR-0050).

use std::ops::ControlFlow;
use std::process::ExitCode;

use schemars::JsonSchema;
use serde::Serialize;

use super::{Unreadable, parse};
use crate::{Document, Row, Sink, Spelling, Stream, stream_to};

/// A nest whose `problem` field marks a problem when it holds a value.
#[derive(Serialize, JsonSchema)]
struct Nest {
    /// The bird that built the nest.
    bird: &'static str,
    /// What is wrong with the nest, or nothing.
    problem: Option<&'static str>,
}

impl Row for Nest {
    const COLUMNS: &'static [&'static str] = &["bird", "problem"];
    const PROBLEM: Option<&'static str> = Some("problem");
}

const NESTS: Document<Stream<Nest>> = Document::new("birds", "nests", 1, "Each nest.");

/// Streams `nests`, reports `failure` after them when it is `Some`, ends the
/// stream without a failure, and returns the exit code and standard error.
fn stream(nests: &[Nest], failure: Option<&Unreadable>) -> (ExitCode, String) {
    let mut output = parse(&[]);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);
    let mut streaming = stream_to(&mut sink, &mut output, Spelling::COLONS, NESTS).unwrap();
    for nest in nests {
        assert_eq!(streaming.item(nest), ControlFlow::Continue(()));
    }
    if let Some(failure) = failure {
        assert_eq!(streaming.failure(failure), ControlFlow::Continue(()));
    }
    let code = streaming.end(Ok::<(), Unreadable>(()));
    (code, String::from_utf8(err).unwrap())
}

const WREN: Nest = Nest {
    bird: "wren",
    problem: None,
};

const ROBIN: Nest = Nest {
    bird: "robin",
    problem: Some("the nest fell"),
};

#[test]
fn a_stream_whose_items_report_a_problem_ends_with_exit_code_1_and_a_note() {
    let (code, err) = stream(&[WREN, ROBIN], None);

    assert_eq!(code, ExitCode::FAILURE);
    assert_eq!(
        err,
        "note: 1 of the rows reports a problem, so the command exits with 1.\n"
    );
}

#[test]
fn a_stream_whose_items_report_no_problem_ends_with_exit_code_0() {
    let (code, err) = stream(&[WREN], None);

    assert_eq!(code, ExitCode::SUCCESS);
    assert_eq!(err, "");
}

#[test]
fn a_stream_failure_decides_the_exit_before_an_item_that_reports_a_problem() {
    let (code, err) = stream(&[ROBIN], Some(&Unreadable(3)));

    assert_eq!(code, ExitCode::FAILURE);
    assert!(!err.contains("reports a problem"), "{err}");
}
