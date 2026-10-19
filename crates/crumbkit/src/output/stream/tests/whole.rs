//! Tests for a stream of a row type that sets `Row::WHOLE` (ADR-0065).

use std::ops::ControlFlow;

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
