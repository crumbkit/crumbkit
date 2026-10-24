//! Tests for a stream of a row type that sets `Row::WHOLE` (ADR-0065,
//! ADR-0138, ADR-0149).

use std::ops::ControlFlow;
use std::process::ExitCode;

use schemars::JsonSchema;
use serde::Serialize;

use super::{Unreadable, parse, stream_to};
use crate::{Document, Row, Sink, Stream};

/// A sighting whose JSON holds every field.
#[derive(Serialize, JsonSchema)]
struct Sighting {
    /// The bird that was seen.
    bird: &'static str,
    /// Where the bird was seen.
    place: &'static str,
}

impl Row for Sighting {
    const COLUMNS: &'static [&'static str] = &["bird"];
    const WHOLE: bool = true;
}

/// Streams `items` as `document` with `flags`, and returns standard output.
fn streamed<R: Row>(flags: &[&str], document: Document<Stream<R>>, items: &[R]) -> String {
    let output = parse(flags);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);
    let Ok(mut streaming) = stream_to(&mut sink, output, document) else {
        panic!("the stream was refused");
    };
    for item in items {
        assert_eq!(streaming.item(item), ControlFlow::Continue(()));
    }
    let _ = streaming.end(Ok::<(), Unreadable>(()));
    String::from_utf8(out).unwrap()
}

#[test]
fn each_json_line_of_a_whole_row_type_holds_every_field_without_full() {
    const SIGHTINGS: Document<Stream<Sighting>> =
        Document::new("birds", "sightings-watch", 1, "Each sighting.");
    let robin = Sighting {
        bird: "robin",
        place: "garden",
    };

    let out = streamed(&["--json"], SIGHTINGS, &[robin]);

    assert_eq!(
        out,
        "{\"schema\":\"birds/sightings-watch/v1\",\"data\":{\"bird\":\"robin\",\"place\":\"garden\"}}\n"
    );
}

/// A sighting with a long column, whose JSON holds every field.
#[derive(Serialize, JsonSchema)]
struct Visit {
    /// The bird that visited.
    bird: &'static str,
    /// Where the bird visited.
    place: &'static str,
}

impl Row for Visit {
    const COLUMNS: &'static [&'static str] = &["bird"];
    const LONG_COLUMNS: &'static [&'static str] = &["place"];
    const WHOLE: bool = true;
}

/// Streams `items` as `document` with `flags`, and returns standard output,
/// standard error and the exit code.
fn streamed_all<R: Row>(
    flags: &[&str],
    document: Document<Stream<R>>,
    items: &[R],
) -> (String, String, ExitCode) {
    let output = parse(flags);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);
    let Ok(mut streaming) = stream_to(&mut sink, output, document) else {
        panic!("the stream was refused");
    };
    for item in items {
        assert_eq!(streaming.item(item), ControlFlow::Continue(()));
    }
    let code = streaming.end(Ok::<(), Unreadable>(()));
    (
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
        code,
    )
}

const VISITS: Document<Stream<Visit>> = Document::new("birds", "visits-watch", 1, "Each visit.");

/// Returns one visit of a robin.
const fn robin() -> Visit {
    Visit {
        bird: "robin",
        place: "garden",
    }
}

#[test]
fn a_stream_of_a_whole_row_type_prints_no_note_for_json_alone() {
    let (_, err, code) = streamed_all(&["--json"], VISITS, &[robin()]);

    assert_eq!(code, ExitCode::SUCCESS);
    assert_eq!(err, "");
}

#[test]
fn a_stream_of_a_whole_row_type_prints_the_json_lines_and_a_note_for_long_with_json() {
    let (json, _, _) = streamed_all(&["--json"], VISITS, &[robin()]);

    let (out, err, code) = streamed_all(&["--json", "--long"], VISITS, &[robin()]);

    assert_eq!(code, ExitCode::SUCCESS, "{err}");
    assert_eq!(out, json);
    assert_eq!(
        err,
        "note: --long adds nothing to --json here; the JSON already holds the whole result.\n"
    );
}

#[test]
fn a_stream_of_a_whole_row_type_prints_the_json_lines_and_a_note_for_full_with_json() {
    let (json, _, _) = streamed_all(&["--json"], VISITS, &[robin()]);

    let (out, err, code) = streamed_all(&["--json", "--full"], VISITS, &[robin()]);

    assert_eq!(code, ExitCode::SUCCESS, "{err}");
    assert_eq!(out, json);
    assert_eq!(
        err,
        "note: --full adds nothing to --json here; the JSON already holds the whole result.\n"
    );
}
