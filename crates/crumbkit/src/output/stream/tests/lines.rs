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

/// A count that serializes as an array, prints whole, and has its own
/// text line.
#[derive(Serialize, JsonSchema)]
struct Count(&'static str, u32);

impl Row for Count {
    const COLUMNS: &'static [&'static str] = &[];
    const WHOLE: bool = true;
    const LINE: Option<&'static str> = Some("<COUNT> <BIRD>");

    fn text_line(&self) -> Option<String> {
        Some(format!("{} {}", self.1, self.0))
    }
}

const COUNTS: Document<Stream<Count>> =
    Document::new("birds", "counts-watch", 1, "Each count as it arrives.");

#[test]
fn a_stream_of_whole_rows_without_columns_that_declare_a_line_prints_each_line() {
    let mut output = parse(&[]);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);
    let code = match stream_to(&mut sink, &mut output, Spelling::COLONS, COUNTS) {
        Ok(mut streaming) => {
            assert_eq!(
                streaming.item(&Count("robin", 2)),
                ControlFlow::Continue(())
            );
            streaming.end(Ok::<(), Error>(()))
        }
        Err(code) => code,
    };

    assert_eq!(code, ExitCode::SUCCESS);
    assert_eq!(String::from_utf8(out).unwrap(), "2 robin\n");
}

/// A sighting that declares a text line but has none, so its text is one
/// line of JSON. Its fields serialize out of name order.
#[derive(Serialize, JsonSchema)]
struct Unplaced {
    /// Where the bird was heard.
    place: &'static str,
    /// The bird that was heard.
    bird: &'static str,
}

impl Row for Unplaced {
    const COLUMNS: &'static [&'static str] = &["bird"];
    const LINE: Option<&'static str> = Some("a <BIRD> at the <PLACE>");

    fn text_line(&self) -> Option<String> {
        None
    }
}

#[test]
fn a_stream_item_without_a_text_line_prints_its_json_line_with_sorted_keys_when_the_guide_sorts_them()
 {
    use clap::{CommandFactory as _, FromArgMatches as _};

    const GUIDE: crate::help::Guide =
        crate::help::Guide::new().json(crate::JsonStyle::new().sorted());
    const UNPLACED: Document<Stream<Unplaced>> =
        Document::new("birds", "unplaced-watch", 1, "Each sighting as it arrives.");
    let mut command = super::Tool::command();
    let matches = command
        .try_get_matches_from_mut(["birds", "watch"])
        .unwrap();
    let mut output = super::Tool::from_arg_matches(&matches).unwrap().output;
    output.set_command(crate::Parsed::guided(command, &GUIDE));
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);
    let mut streaming = stream_to(&mut sink, &mut output, Spelling::COLONS, UNPLACED).unwrap();

    let unplaced = Unplaced {
        place: "garden",
        bird: "robin",
    };
    assert_eq!(streaming.item(&unplaced), ControlFlow::Continue(()));
    let _ = streaming.end(Ok::<(), Error>(()));

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"bird\":\"robin\",\"place\":\"garden\"}\n"
    );
}
