//! Tests for a stream of a row type that sets `Row::WHOLE` (ADR-0065,
//! ADR-0138).

use std::ops::ControlFlow;
use std::process::ExitCode;

use schemars::JsonSchema;
use serde::Serialize;

use super::{Unreadable, parse, stream_to};
use crate::twins::assert_one_word_apart;
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

#[test]
fn a_stream_of_a_whole_row_type_refuses_long_with_json_before_anything_prints() {
    const VISITS: Document<Stream<Visit>> =
        Document::new("birds", "visits-watch", 1, "Each visit.");
    assert_one_word_apart(&["--json", "--long"], &["--json"]);
    let robin = Visit {
        bird: "robin",
        place: "garden",
    };
    assert!(streamed(&["--json"], VISITS, &[robin]).starts_with('{'));
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);

    let refused = stream_to(&mut sink, parse(&["--json", "--long"]), VISITS);

    assert_eq!(refused.err(), Some(ExitCode::from(2)));
    let document: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(document["code"], "birds::cli::usage");
    assert_eq!(
        document["recovery"]["text"],
        "drop --long, because --json already prints every field of each row"
    );
}
