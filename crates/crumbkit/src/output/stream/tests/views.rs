//! Tests for the view flags that a stream refuses because its document
//! cannot use them.

use std::process::ExitCode;

use super::{SIGHTINGS, Unreadable, robin, run, run_budgeted_as};
use crate::Envelope;

#[test]
fn full_with_json_on_a_stream_without_the_envelope_prints_the_json_lines_and_a_note() {
    let unwrapped = SIGHTINGS.envelope(Envelope::Without);
    let json = run(&["--json"], unwrapped, &[robin()], None, Ok(()));

    let streamed = run(&["--json", "--full"], unwrapped, &[robin()], None, Ok(()));

    assert_eq!(streamed.code, ExitCode::SUCCESS, "{}", streamed.err);
    assert_eq!(streamed.out, json.out);
    assert_eq!(json.err, "");
    assert_eq!(
        streamed.err,
        "note: --full adds nothing to --json here; the JSON already holds the whole result.\n"
    );
}

#[test]
fn long_with_json_on_a_stream_without_the_envelope_prints_the_json_lines_and_a_note() {
    let unwrapped = SIGHTINGS.envelope(Envelope::Without);
    let json = run(&["--json"], unwrapped, &[robin()], None, Ok(()));

    let streamed = run(&["--json", "--long"], unwrapped, &[robin()], None, Ok(()));

    assert_eq!(streamed.code, ExitCode::SUCCESS, "{}", streamed.err);
    assert_eq!(streamed.out, json.out);
    assert_eq!(
        streamed.err,
        "note: --long adds nothing to --json here; the JSON already holds the whole result.\n"
    );
}

/// The note that `--full` adds nothing to the JSON.
const FULL_NOTE: &str =
    "note: --full adds nothing to --json here; the JSON already holds the whole result.\n";

#[test]
fn the_note_of_a_stream_follows_a_failure_that_the_stream_printed_before_it_ended() {
    let unwrapped = SIGHTINGS.envelope(Envelope::Without);
    let failure = Unreadable(3);
    let json = run(
        &["--json"],
        unwrapped,
        &[robin(), robin()],
        Some(&failure),
        Ok(()),
    );

    let streamed = run(
        &["--json", "--full"],
        unwrapped,
        &[robin(), robin()],
        Some(&failure),
        Ok(()),
    );

    assert_eq!(streamed.code, json.code);
    assert_eq!(streamed.out, json.out);
    assert_eq!(streamed.err, format!("{}{FULL_NOTE}", json.err));
}

#[test]
fn a_stream_that_ends_with_an_err_still_prints_the_note_after_its_failure() {
    let unwrapped = SIGHTINGS.envelope(Envelope::Without);
    let json = run(&["--json"], unwrapped, &[robin()], None, Err(Unreadable(3)));

    let streamed = run(
        &["--json", "--full"],
        unwrapped,
        &[robin()],
        None,
        Err(Unreadable(3)),
    );

    assert_eq!(streamed.code, json.code);
    assert_eq!(streamed.out, json.out);
    assert!(json.err.starts_with("error: "), "{}", json.err);
    assert_eq!(streamed.err, format!("{}{FULL_NOTE}", json.err));
}

#[test]
fn a_stream_that_the_output_budget_stopped_still_prints_the_note() {
    let unwrapped = SIGHTINGS.envelope(Envelope::Without);
    let (json, _) = run_budgeted_as(&["--json"], 1000, unwrapped);

    let (streamed, _) = run_budgeted_as(&["--json", "--full"], 1000, unwrapped);

    assert_eq!(streamed.code, ExitCode::FAILURE);
    assert_eq!(streamed.out, json.out);
    assert!(json.err.contains("output budget"), "{}", json.err);
    assert_eq!(streamed.err, format!("{}{FULL_NOTE}", json.err));
}
