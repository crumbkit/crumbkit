//! Tests for the note of a stream that ends with no rows (ADR-0050).

use std::process::ExitCode;

use super::{SIGHTINGS, Unreadable, robin, run, run_ok};

#[test]
fn a_stream_that_ends_with_no_rows_prints_its_header_and_the_note_on_standard_error() {
    let streamed = run_ok(&[], &[]);

    assert_eq!(streamed.code, ExitCode::SUCCESS);
    assert_eq!(streamed.out, "NUMBER  BIRD\n");
    assert_eq!(streamed.err, "note: no rows\n");
}

#[test]
fn a_stream_document_sets_its_own_wording_of_the_note() {
    let document = SIGHTINGS.when_empty("no birds were seen");

    let streamed = run(&[], document, &[], None, Ok(()));

    assert_eq!(streamed.err, "note: no birds were seen\n");
}

#[test]
fn a_json_stream_with_no_rows_prints_nothing_and_the_note_on_standard_error() {
    let streamed = run_ok(&["--json"], &[]);

    assert_eq!(streamed.code, ExitCode::SUCCESS);
    assert_eq!(streamed.out, "");
    assert_eq!(streamed.err, "note: no rows\n");
}

#[test]
fn the_note_comes_before_the_line_of_a_dry_run() {
    let streamed = run_ok(&["--dry-run"], &[]);

    assert_eq!(streamed.out, "NUMBER  BIRD\n");
    assert_eq!(
        streamed.err,
        "note: no rows\nThis was a dry run, so nothing changed.\n"
    );
}

#[test]
fn a_stream_with_rows_prints_no_note() {
    let streamed = run_ok(&[], &[robin()]);

    assert!(!streamed.err.contains("note:"), "{}", streamed.err);
}

#[test]
fn a_stream_that_ends_with_a_failure_and_no_rows_prints_no_note() {
    let streamed = run(&[], SIGHTINGS, &[], None, Err(Unreadable(3)));

    assert_eq!(streamed.out, "NUMBER  BIRD\n");
    assert!(!streamed.err.contains("note:"), "{}", streamed.err);
}
