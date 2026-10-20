//! Tests for a stream whose rows print their children as indented lines
//! under them (ADR-0120).

use std::ops::ControlFlow;
use std::process::ExitCode;

use schemars::JsonSchema;
use serde::Serialize;

use super::{Streamed, parse};
use crate::{Document, Error, Row, Sink, Stream, stream_to};

/// A nest and the eggs in it, each of which prints under the nest.
#[derive(Serialize, JsonSchema)]
struct Nest {
    /// The bird that built the nest.
    bird: &'static str,
    /// Where the nest is.
    place: &'static str,
    /// The colour of each egg in the nest.
    eggs: Vec<&'static str>,
}

impl Row for Nest {
    const COLUMNS: &'static [&'static str] = &["bird"];
    const LONG_COLUMNS: &'static [&'static str] = &["place"];
    const CHILDREN: Option<&'static str> = Some("eggs");

    fn child_lines(&self) -> Vec<String> {
        self.eggs.iter().map(|egg| format!("egg {egg}")).collect()
    }
}

const NESTS: Document<Stream<Nest>> =
    Document::new("birds", "nests-watch", 1, "Each nest as it is found.");

/// Streams a robin's nest with two eggs and an empty heron's nest with
/// `flags`, and ends without a failure.
fn run(flags: &[&str]) -> Streamed {
    let output = parse(flags);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);
    let nests = [
        Nest {
            bird: "robin",
            place: "hedge",
            eggs: vec!["blue", "blue\u{200b}"],
        },
        Nest {
            bird: "heron",
            place: "reeds",
            eggs: Vec::new(),
        },
    ];
    let code = match stream_to(&mut sink, output, NESTS) {
        Ok(mut streaming) => {
            for nest in &nests {
                assert_eq!(streaming.item(nest), ControlFlow::Continue(()));
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
fn a_stream_prints_each_item_s_child_lines_after_the_item_s_line() {
    let streamed = run(&[]);

    assert_eq!(streamed.code, ExitCode::SUCCESS, "{}", streamed.err);
    assert_eq!(
        streamed.out,
        "BIRD\nrobin\n  egg blue\n  egg blue\\u{200b}\nheron\n"
    );
}

#[test]
fn a_stream_with_long_prints_the_long_columns_and_the_child_lines() {
    let streamed = run(&["--long"]);

    assert_eq!(
        streamed.out,
        "BIRD  PLACE\nrobin  hedge\n  egg blue\n  egg blue\\u{200b}\nheron  reeds\n"
    );
}

#[test]
fn a_stream_with_fields_prints_no_child_lines() {
    let streamed = run(&["--fields", "place"]);

    assert_eq!(streamed.out, "PLACE\nhedge\nreeds\n");
}

#[test]
fn each_compact_json_line_of_a_stream_holds_the_children_field_after_the_columns() {
    let streamed = run(&["--json"]);

    let first = streamed.out.lines().next().unwrap();
    assert_eq!(
        first,
        "{\"schema\":\"birds/nests-watch/v1\",\"data\":{\"bird\":\"robin\",\"eggs\":[\"blue\",\"blue\u{200b}\"]}}"
    );
}

/// A sighting whose text is its own line, with the birds seen with it under
/// that line.
#[derive(Serialize, JsonSchema)]
struct Flock {
    /// The bird that leads the flock.
    leader: &'static str,
    /// The other birds in the flock.
    others: Vec<&'static str>,
}

impl Row for Flock {
    const COLUMNS: &'static [&'static str] = &["leader"];
    const LINE: Option<&'static str> = Some("a flock led by a <LEADER>");
    const CHILDREN: Option<&'static str> = Some("others");

    fn text_line(&self) -> Option<String> {
        Some(format!("a flock led by a {}", self.leader))
    }

    fn child_lines(&self) -> Vec<String> {
        self.others.iter().map(|&other| other.to_owned()).collect()
    }
}

#[test]
fn a_stream_of_rows_with_their_own_text_line_prints_each_item_s_child_lines_after_that_line() {
    const FLOCKS: Document<Stream<Flock>> =
        Document::new("birds", "flocks-watch", 1, "Each flock as it arrives.");
    let output = parse(&[]);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);
    let flock = Flock {
        leader: "goose",
        others: vec!["swan", "duck"],
    };
    let code = match stream_to(&mut sink, output, FLOCKS) {
        Ok(mut streaming) => {
            assert_eq!(streaming.item(&flock), ControlFlow::Continue(()));
            streaming.end(Ok::<(), Error>(()))
        }
        Err(code) => code,
    };

    assert_eq!(code, ExitCode::SUCCESS);
    assert_eq!(
        String::from_utf8(out).unwrap(),
        "a flock led by a goose\n  swan\n  duck\n"
    );
}
