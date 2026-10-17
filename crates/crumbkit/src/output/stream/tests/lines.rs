//! Tests for a stream whose rows print their own text line (ADR-0064).

use std::ops::ControlFlow;
use std::process::ExitCode;

use schemars::JsonSchema;
use serde::Serialize;

use super::{Streamed, parse};
use crate::{Document, Error, Row, Sink, Spelling, Stream, stream_to};

/// A sighting whose text is one sentence.
#[derive(Serialize, JsonSchema)]
struct Seen {
    /// The bird that was seen.
    bird: &'static str,
    /// Where the bird was seen.
    place: &'static str,
}

impl Row for Seen {
    const COLUMNS: &'static [&'static str] = &["bird"];
    const LONG_COLUMNS: &'static [&'static str] = &["place"];
    const LINE: Option<&'static str> = Some("a <BIRD> at the <PLACE>");

    fn text_line(&self) -> Option<String> {
        Some(format!("a {} at the {}", self.bird, self.place))
    }
}

const SEEN: Document<Stream<Seen>> =
    Document::new("birds", "seen-watch", 1, "Each sighting as it arrives.");

/// Streams a robin and a heron with `flags` and ends without a failure.
fn run(flags: &[&str]) -> Streamed {
    let mut output = parse(flags);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);
    let code = match stream_to(&mut sink, &mut output, Spelling::COLONS, SEEN) {
        Ok(mut streaming) => {
            for (bird, place) in [("robin", "garden"), ("heron", "pond")] {
                assert_eq!(
                    streaming.item(&Seen { bird, place }),
                    ControlFlow::Continue(())
                );
            }
            streaming.end(Ok::<(), Error>(()))
        }
        Err(code) => code,
    };
    Streamed {
        code,
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

#[test]
fn a_stream_of_rows_with_their_own_text_line_prints_no_header_and_each_item_s_line() {
    let streamed = run(&[]);

    assert_eq!(streamed.code, ExitCode::SUCCESS, "{}", streamed.err);
    assert_eq!(streamed.out, "a robin at the garden\na heron at the pond\n");
}

#[test]
fn a_stream_of_rows_with_their_own_text_line_prints_the_table_with_long() {
    let streamed = run(&["--long"]);

    assert_eq!(streamed.out, "BIRD  PLACE\nrobin  garden\nheron  pond\n");
}

#[test]
fn a_stream_of_rows_with_their_own_text_line_prints_the_table_with_fields() {
    let streamed = run(&["--fields", "place"]);

    assert_eq!(streamed.out, "PLACE\ngarden\npond\n");
}
