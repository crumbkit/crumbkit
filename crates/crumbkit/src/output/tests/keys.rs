//! Tests for how `finish_to` prints a document with the tool's own
//! envelope keys (ADR-0059).

use std::fmt;
use std::process::ExitCode;

use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Value, json};

use super::{Planet, finish_with, mercury, saturn};
use crate::{Document, Listing, Prose, Row};

const ENTRIES: Document<Listing<Planet>> =
    Document::new("solar", "planet-listing", 1, "The test rows.").with_keys("$type", "entries");

/// Returns a listing of Mercury and Saturn out of 8 planets.
fn two_of_eight() -> Listing<Planet> {
    Listing::new(vec![mercury(), saturn()], 8)
}

#[test]
fn a_listing_prints_its_id_and_rows_under_the_declared_keys_and_keeps_total_and_truncated() {
    let finished = finish_with(&["--json"], ENTRIES, two_of_eight());

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(
        finished.out,
        concat!(
            r#"{"$type":"solar/planet-listing/v1","entries":[{"name":"Mercury","moons":0},{"name":"Saturn","moons":274}],"#,
            r#""total":8,"truncated":true}"#,
            "\n"
        )
    );
}

#[test]
fn the_type_key_holds_the_declared_type_value_in_place_of_the_id() {
    let finished = finish_with(&["--json"], ENTRIES.with_type("planets"), two_of_eight());

    let printed: Value = serde_json::from_str(&finished.out).unwrap();
    assert_eq!(printed["$type"], "planets");
}

#[test]
fn the_view_flags_shape_the_rows_under_the_data_key_as_under_crumbkits_envelope() {
    let long = finish_with(&["--json", "--long"], ENTRIES, two_of_eight());
    let full = finish_with(&["--json", "--full"], ENTRIES, two_of_eight());
    let fields = finish_with(&["--json", "--fields", "moons"], ENTRIES, two_of_eight());

    let entries = |out: &str| serde_json::from_str::<Value>(out).unwrap()["entries"][1].clone();
    assert_eq!(
        entries(&long.out),
        json!({"name": "Saturn", "moons": 274, "ringed": true})
    );
    assert_eq!(
        entries(&full.out),
        json!({"name": "Saturn", "moons": 274, "ringed": true, "au": 9.54})
    );
    assert_eq!(entries(&fields.out), json!({"moons": 274}));
}

/// A row whose every field is a compact column and that has no long
/// columns, as a tool whose readers expect every field declares it.
#[derive(Serialize, JsonSchema)]
struct Moon {
    name: &'static str,
    planet: &'static str,
}

impl Row for Moon {
    const COLUMNS: &'static [&'static str] = &["name", "planet"];
}

const MOONS: Document<Vec<Moon>> =
    Document::new("solar", "moons", 1, "The test moons.").with_keys("$type", "rows");

fn titan() -> Vec<Moon> {
    vec![Moon {
        name: "Titan",
        planet: "Saturn",
    }]
}

#[test]
fn a_row_whose_every_field_is_compact_prints_every_field_without_full() {
    let compact = finish_with(&["--json"], MOONS, titan());
    let full = finish_with(&["--json", "--full"], MOONS, titan());

    assert_eq!(
        compact.out,
        "{\"$type\":\"solar/moons/v1\",\"rows\":[{\"name\":\"Titan\",\"planet\":\"Saturn\"}]}\n"
    );
    assert_eq!(compact.out, full.out);
}

#[test]
fn long_is_refused_for_a_row_without_long_columns() {
    let refused = finish_with(&["--json", "--long"], MOONS, titan());
    let twin = finish_with(&["--json"], MOONS, titan());

    assert_eq!(refused.code, ExitCode::from(2));
    assert!(refused.out.contains("--long"), "{}", refused.out);
    assert_eq!(twin.code, ExitCode::SUCCESS);
}

/// The fields that a listing of planets carries beside its rows.
#[derive(Serialize, JsonSchema)]
struct Location {
    location: Value,
    degraded: bool,
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Planets around the Sun.")
    }
}

const LOCATED: Document<Listing<Planet, Location>> =
    Document::new("solar", "located", 1, "The test rows.")
        .with_keys("$type", "entries")
        .summary_at_top();

fn located(location: Value) -> Listing<Planet, Location> {
    two_of_eight().summary(Location {
        location,
        degraded: false,
    })
}

#[test]
fn a_summary_at_the_top_prints_its_fields_beside_total_and_truncated() {
    let finished = finish_with(&["--json"], LOCATED, located(json!({"star": "Sun"})));

    assert_eq!(finished.code, ExitCode::SUCCESS);
    let printed: Value = serde_json::from_str(&finished.out).unwrap();
    assert_eq!(printed["location"], json!({"star": "Sun"}));
    assert_eq!(printed["degraded"], false);
    assert_eq!(printed["total"], 8);
    assert_eq!(printed["truncated"], true);
    assert!(printed.get("summary").is_none(), "{}", finished.out);
}

#[test]
fn a_summary_at_the_top_still_prints_as_a_line_above_the_table_in_text() {
    let finished = finish_with(&[], LOCATED, located(json!(null)));

    assert!(
        finished.out.starts_with("Planets around the Sun.\nNAME"),
        "{}",
        finished.out
    );
}

/// A summary with a field that takes a name crumbkit writes.
#[derive(Serialize, JsonSchema)]
struct Clash {
    total: u32,
}

impl fmt::Display for Clash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("A clash.")
    }
}

#[test]
fn a_summary_field_with_a_name_that_crumbkit_writes_fails_the_run_and_prints_no_document() {
    const CLASHING: Document<Listing<Planet, Clash>> =
        Document::new("solar", "clashing", 1, "The test rows.")
            .with_keys("$type", "entries")
            .summary_at_top();
    let finished = finish_with(
        &["--json"],
        CLASHING,
        two_of_eight().summary(Clash { total: 1 }),
    );

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert_eq!(finished.out, "");
    assert!(finished.err.contains("`total`"), "{}", finished.err);
}

/// The value of a prose result.
#[derive(Serialize, JsonSchema)]
struct Count {
    planets: u32,
}

impl fmt::Display for Count {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} planets", self.planets)
    }
}

#[test]
fn a_prose_result_holds_its_value_under_the_data_key_and_keeps_failures() {
    const COUNT: Document<Prose<Count, crate::Error>> =
        Document::new("solar", "count", 1, "The count.").with_keys("kind", "answer");
    let finished = finish_with(&["--json"], COUNT, Prose::new(Count { planets: 8 }));

    assert_eq!(
        finished.out,
        "{\"kind\":\"solar/count/v1\",\"answer\":{\"planets\":8},\"failures\":[]}\n"
    );
}

#[test]
fn with_keys_rejects_each_name_that_crumbkit_writes_beside_the_result() {
    for name in crate::output::envelope::WRITTEN {
        let declared = std::panic::catch_unwind(|| {
            Document::<Listing<Planet>>::new("solar", "planet-listing", 1, "The test rows.")
                .with_keys("$type", name)
        });
        assert!(declared.is_err(), "with_keys accepted {name}");
    }
}
