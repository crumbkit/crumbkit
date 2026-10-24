//! Tests for the view flags that a stream refuses because its document
//! cannot use them.

use std::process::ExitCode;

use super::{SIGHTINGS, robin, run};
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
