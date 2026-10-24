//! Tests for how `finish_to` prints the rows of a row type that sets
//! `Row::WHOLE`: every field with `--json`, and its table in text when it
//! has columns (ADR-0065).

use std::process::ExitCode;

use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Value, json};

use super::finish_with;
use crate::twins::assert_one_word_apart;
use crate::{Document, Listing, Row};

/// A moon whose optional field is left out of its JSON when it holds
/// nothing, as a tool whose readers expect whole rows serializes it.
#[derive(Serialize, JsonSchema)]
struct Moon {
    /// The moon's name.
    name: &'static str,
    /// The moon's discoverer, when one is known.
    #[serde(skip_serializing_if = "Option::is_none")]
    discoverer: Option<&'static str>,
    /// The moon's planet.
    planet: &'static str,
}

impl Row for Moon {
    const COLUMNS: &'static [&'static str] = &["name", "discoverer"];
    const WHOLE: bool = true;
}

const MOONS: Document<Listing<Moon>> = Document::new("solar", "moons", 1, "The test moons.");

/// Returns a listing of Io, whose discoverer is known, and the Moon, whose
/// discoverer is not.
fn moons() -> Listing<Moon> {
    let moons = vec![
        Moon {
            name: "Io",
            discoverer: Some("Galileo"),
            planet: "Jupiter",
        },
        Moon {
            name: "Moon",
            discoverer: None,
            planet: "Earth",
        },
    ];
    Listing::new(moons, 2)
}

#[test]
fn json_of_a_whole_row_type_prints_each_row_as_it_serializes_without_full() {
    let finished = finish_with(&["--json"], MOONS, moons());

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"solar/moons/v1","data":[{"name":"Io","discoverer":"Galileo","planet":"Jupiter"},"#,
            r#"{"name":"Moon","planet":"Earth"}],"total":2,"truncated":false}"#,
            "\n"
        )
    );
}

#[test]
fn text_of_a_whole_row_type_with_columns_is_the_table_of_its_columns() {
    let finished = finish_with(&[], MOONS, moons());

    assert_eq!(finished.out, "NAME  DISCOVERER\nIo    Galileo\nMoon  -\n");
}

#[test]
fn fields_selects_named_fields_of_a_whole_row_and_prints_null_for_a_field_the_row_leaves_out() {
    let finished = finish_with(&["--json", "--fields", "discoverer"], MOONS, moons());

    let printed: Value = serde_json::from_str(&finished.out).unwrap();
    assert_eq!(
        printed["data"],
        json!([{"discoverer": "Galileo"}, {"discoverer": null}])
    );
}

/// The note that `--long` with `--json` prints for a whole row type.
const LONG_NOTE: &str =
    "note: --long adds nothing to --json here; the JSON already holds the whole result.\n";

/// The note that `--full` with `--json` prints for a whole row type.
const FULL_NOTE: &str =
    "note: --full adds nothing to --json here; the JSON already holds the whole result.\n";

#[test]
fn json_alone_for_a_whole_row_type_prints_no_note() {
    let finished = finish_with(&["--json"], MOONS, moons());

    assert_eq!(finished.err, "");
}

#[test]
fn full_with_json_for_a_whole_row_type_prints_the_json_document_and_a_note() {
    let json = finish_with(&["--json"], MOONS, moons());

    let finished = finish_with(&["--json", "--full"], MOONS, moons());

    assert_eq!(finished.code, ExitCode::SUCCESS, "{}", finished.err);
    assert_eq!(finished.out, json.out);
    assert_eq!(finished.err, FULL_NOTE);
}

#[test]
fn long_with_json_for_a_whole_row_type_without_long_columns_prints_the_json_document_and_a_note() {
    let json = finish_with(&["--json"], MOONS, moons());

    let finished = finish_with(&["--json", "--long"], MOONS, moons());

    assert_eq!(finished.code, ExitCode::SUCCESS, "{}", finished.err);
    assert_eq!(finished.out, json.out);
    assert_eq!(finished.err, LONG_NOTE);
}

#[test]
fn long_and_full_with_json_for_a_whole_row_type_print_one_note_that_names_full() {
    let json = finish_with(&["--json"], MOONS, moons());

    let finished = finish_with(&["--json", "--long", "--full"], MOONS, moons());

    assert_eq!(finished.code, ExitCode::SUCCESS, "{}", finished.err);
    assert_eq!(finished.out, json.out);
    assert_eq!(finished.err, FULL_NOTE);
}

/// A query result row, which serializes as an array of its values and so
/// has no fields to name as columns.
#[derive(Serialize, JsonSchema)]
struct Answer(&'static str, u32);

impl Row for Answer {
    const COLUMNS: &'static [&'static str] = &[];
    const LINE: Option<&'static str> = Some("<NAME> <YEAR>");
    const WHOLE: bool = true;
}

const ANSWERS: Document<Listing<Answer>> =
    Document::new("solar", "answers", 1, "The test answers.");

/// Returns a page of two answers from a source that holds more.
fn answers() -> Listing<Answer> {
    Listing::uncounted(vec![Answer("Io", 1610), Answer("Titan", 1655)], true)
}

#[test]
fn json_of_a_whole_row_type_without_columns_prints_each_row_as_it_serializes() {
    let finished = finish_with(&["--json"], ANSWERS, answers());

    assert_eq!(
        finished.out,
        "{\"schema\":\"solar/answers/v1\",\"data\":[[\"Io\",1610],[\"Titan\",1655]],\"truncated\":true}\n"
    );
}

#[test]
fn finish_refuses_fields_for_a_whole_row_type_without_columns() {
    assert_one_word_apart(&["--fields=name"], &[]);
    let twin = finish_with(&[], ANSWERS, answers());
    assert_eq!(twin.code, ExitCode::SUCCESS, "{}", twin.err);

    let finished = finish_with(&["--fields=name"], ANSWERS, answers());

    assert_eq!(finished.code, ExitCode::from(2));
    assert_eq!(finished.out, "");
    assert!(
        finished.err.starts_with(
            "error: --fields selects the columns of a table, and this command's rows have no columns\n  help:  use --json to read every field of each row\n"
        ),
        "{}",
        finished.err
    );
}

#[test]
fn long_with_json_for_a_whole_row_type_without_columns_prints_the_json_document_and_a_note() {
    let json = finish_with(&["--json"], ANSWERS, answers());

    let finished = finish_with(&["--json", "--long"], ANSWERS, answers());

    assert_eq!(finished.code, ExitCode::SUCCESS, "{}", finished.err);
    assert_eq!(finished.out, json.out);
    assert_eq!(finished.err, LONG_NOTE);
}

#[test]
fn long_without_json_for_a_whole_row_type_without_columns_is_still_refused() {
    assert_one_word_apart(&["--long"], &["--json", "--long"]);

    let finished = finish_with(&["--long"], ANSWERS, answers());

    assert_eq!(finished.code, ExitCode::from(2));
    assert!(
        finished
            .err
            .starts_with("error: --long adds the long columns of a table, and this command's rows have no columns\n"),
        "{}",
        finished.err
    );
}
