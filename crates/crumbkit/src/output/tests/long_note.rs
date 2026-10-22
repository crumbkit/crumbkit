//! Tests for a listing's long note, which the text prints on standard error
//! under `--long` only, and for a listing document that declares one, whose
//! command accepts `--long` although its rows have no long columns
//! (ADR-0139).

use std::process::ExitCode;

use schemars::JsonSchema;
use serde::Serialize;

use super::{PLANET_LISTING, Planet, finish_with, mercury, saturn};
use crate::twins::assert_one_word_apart;
use crate::{Call, Document, Listing, Row};

/// The disk space that one directory takes, without long columns.
#[derive(Serialize, JsonSchema)]
struct Usage {
    /// The directory that was measured.
    path: &'static str,
    /// The bytes that its files take on disk.
    bytes: u64,
}

impl Row for Usage {
    const COLUMNS: &'static [&'static str] = &["path", "bytes"];
}

const USAGE: Document<Listing<Usage>> =
    Document::new("du", "usage", 1, "The space that each directory takes.").long_noted();
const UNNOTED_USAGE: Document<Listing<Usage>> =
    Document::new("du", "usage", 1, "The space that each directory takes.");

const HOW: &str = "Sizes are blocks on disk, and a file with two hard links counts once.";

/// Returns the usage of two directories, with a long note that says how
/// the sizes were measured.
fn usage() -> Listing<Usage> {
    let rows = vec![
        Usage {
            path: "src",
            bytes: 4096,
        },
        Usage {
            path: "docs",
            bytes: 512,
        },
    ];
    Listing::from_all(rows, None)
        .described("directories")
        .long_note(HOW)
}

/// Returns a listing of Mercury and Saturn out of 8 planets, with a note, a
/// long note and a next step.
fn noted_planets() -> Listing<Planet> {
    Listing::new(vec![mercury(), saturn()], 8)
        .described("planets")
        .note("Pluto is not a planet.")
        .long_note("Moons are counted from the 2023 survey.")
        .next(Call::new(&["show"]).arg("names"), "shows planets by name.")
}

#[test]
fn under_long_the_long_note_prints_on_standard_error_after_the_note_and_before_the_next_line() {
    let finished = finish_with(&["--long"], PLANET_LISTING, noted_planets());

    assert_eq!(finished.code, ExitCode::SUCCESS, "{}", finished.err);
    assert_eq!(
        finished.out,
        "NAME     MOONS  RINGED\nMercury  0      false\nSaturn   274    true\n"
    );
    assert_eq!(
        finished.err,
        "\
Showing 2 of 8 planets. --all shows every one.
Pluto is not a planet.
Moons are counted from the 2023 survey.
Next: tool show <NAME>... shows planets by name.
"
    );
}

#[test]
fn the_default_text_leaves_out_the_long_note() {
    let finished = finish_with(&[], PLANET_LISTING, noted_planets());

    assert_eq!(
        finished.err,
        "\
Showing 2 of 8 planets. --all shows every one.
Pluto is not a planet.
Next: tool show <NAME>... shows planets by name.
"
    );
}

#[test]
fn fields_leaves_out_the_long_note() {
    let finished = finish_with(&["--fields", "name"], PLANET_LISTING, noted_planets());

    assert!(!finished.err.contains("survey"), "{}", finished.err);
}

#[test]
fn json_with_long_leaves_out_the_long_note_and_every_other_note() {
    let finished = finish_with(&["--json", "--long"], PLANET_LISTING, noted_planets());

    assert_eq!(finished.code, ExitCode::SUCCESS, "{}", finished.err);
    assert_eq!(finished.err, "");
    assert!(!finished.out.contains("survey"), "{}", finished.out);
}

#[test]
fn a_document_that_declares_a_long_note_accepts_long_although_its_rows_have_no_long_columns() {
    let finished = finish_with(&["--long"], USAGE, usage());

    assert_eq!(finished.code, ExitCode::SUCCESS, "{}", finished.err);
    assert_eq!(finished.out, "PATH  BYTES\nsrc   4096\ndocs  512\n");
    assert_eq!(finished.err, format!("{HOW}\n"));
}

#[test]
fn json_with_long_on_a_document_that_declares_a_long_note_prints_the_document_of_json_alone() {
    let long = finish_with(&["--json", "--long"], USAGE, usage());
    let json = finish_with(&["--json"], USAGE, usage());

    assert_eq!(long.code, ExitCode::SUCCESS, "{}", long.err);
    assert_eq!(long.out, json.out);
    assert_eq!(long.err, "");
}

#[test]
fn a_document_without_a_long_note_refuses_long_when_its_rows_have_no_long_columns() {
    assert_one_word_apart(&["--long"], &[]);
    let twin = finish_with(&[], UNNOTED_USAGE, usage());
    assert_eq!(twin.code, ExitCode::SUCCESS, "{}", twin.err);

    let finished = finish_with(&["--long"], UNNOTED_USAGE, usage());

    assert_eq!(finished.code, ExitCode::from(2));
    assert!(
        finished
            .err
            .contains("--long adds the long columns of a table, and this command's rows have none"),
        "{}",
        finished.err
    );
}
