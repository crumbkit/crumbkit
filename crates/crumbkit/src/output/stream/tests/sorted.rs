//! Tests for a stream of a tool whose guide sets `JsonStyle::sorted`
//! (ADR-0067).

use std::ops::ControlFlow;

use clap::{CommandFactory as _, FromArgMatches as _};

use super::{SIGHTINGS, Tool, Unreadable, robin, stream_to};
use crate::help::Guide;
use crate::{JsonStyle, Parsed, Sink, Spelling};

#[test]
fn each_line_of_a_sorted_stream_and_its_failure_prints_sorted_keys_on_one_line() {
    const GUIDE: Guide =
        Guide::new().json(JsonStyle::new().sorted().pretty("the readers expect it"));
    let mut command = Tool::command();
    let matches = command
        .try_get_matches_from_mut(["birds", "watch", "--json", "--full"])
        .unwrap();
    let mut output = Tool::from_arg_matches(&matches).unwrap().output;
    output.set_command(Parsed::guided(command, &GUIDE));
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);
    let mut streaming = stream_to(&mut sink, &mut output, Spelling::COLONS, SIGHTINGS).unwrap();

    assert_eq!(streaming.item(&robin()), ControlFlow::Continue(()));
    assert_eq!(streaming.failure(&Unreadable(4)), ControlFlow::Continue(()));
    let _ = streaming.end(Ok::<(), Unreadable>(()));

    let out = String::from_utf8(out).unwrap();
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(
        lines[0],
        r#"{"data":{"bird":"robin","note":"singing","number":1,"place":"garden"},"schema":"birds/sightings-watch/v1"}"#
    );
    assert!(
        lines[1].starts_with(r#"{"code":"birds::sightings::unreadable","message":"line 4 of the log cannot be read","recovery":{"call":"#),
        "{out}"
    );
    assert_eq!(lines.len(), 2, "{out}");
}
