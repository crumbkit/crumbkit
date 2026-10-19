//! Tests for prose results. crumbkit's tests check how `finish_to` prints
//! them.

use std::fmt;

use schemars::{JsonSchema, SchemaGenerator};
use serde::Serialize;
use serde_json::json;

use super::Prose;
use crate::{Batch, Call, Code, Failure, Listing, NoRow, Recovery, Row, Rows, Shape};

const NOT_FOUND: Code = Code::new("solar", "planets", "not_found");

/// The answer of a command that counts planets.
#[derive(Serialize, JsonSchema)]
struct Count {
    /// The number of planets.
    planets: u32,
}

impl fmt::Display for Count {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "The solar system has {} planets.", self.planets)
    }
}

#[derive(Serialize, JsonSchema)]
struct Planet {
    name: &'static str,
}

impl Row for Planet {
    const COLUMNS: &'static [&'static str] = &["name"];
}

/// A failure that names a planet the solar system lacks.
#[derive(Debug)]
struct Missing(&'static str);

impl fmt::Display for Missing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the solar system has no planet {}", self.0)
    }
}

impl std::error::Error for Missing {}

impl Failure for Missing {
    fn code(&self) -> &'static Code {
        &NOT_FOUND
    }

    fn recovery(&self) -> Recovery {
        Recovery::Run(Call::new(&["list"]))
    }
}

#[test]
fn a_prose_result_serializes_as_its_value() {
    let prose: Prose<Count> = Prose::new(Count { planets: 8 });

    assert_eq!(serde_json::to_value(&prose).unwrap(), json!({"planets": 8}));
}

#[test]
fn a_prose_result_has_no_rows_and_returns_its_text() {
    let prose: Prose<Count> = Prose::new(Count { planets: 8 });

    assert_eq!(prose.rows().len(), 0);
    assert_eq!(
        prose.prose().map(ToString::to_string).as_deref(),
        Some("The solar system has 8 planets.")
    );
    assert_eq!(<Prose<Count> as Rows>::SHAPE, Shape::Prose);
}

#[test]
fn new_holds_no_failures_and_with_failures_holds_the_failures_in_order() {
    let alone: Prose<Count, Missing> = Prose::new(Count { planets: 8 });
    let failed = Prose::with_failures(
        Count { planets: 8 },
        vec![Missing("Vulcan"), Missing("Nibiru")],
    );

    assert_eq!(alone.failures().map(|list| list.len()), Some(0));
    let failures: Vec<String> = failed
        .failures()
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        failures,
        [
            "the solar system has no planet Vulcan",
            "the solar system has no planet Nibiru"
        ]
    );
}

#[test]
fn only_a_prose_result_returns_a_text_and_the_schema_of_its_value() {
    let mut generator = SchemaGenerator::default();
    let listing = Listing::new(vec![Planet { name: "Saturn" }], 1);
    let batch: Batch<Planet, Missing> = Batch::new(&["show"], "names", &["Saturn"]);

    assert!(Planet { name: "Saturn" }.prose().is_none());
    assert!(listing.prose().is_none());
    assert!(batch.prose().is_none());
    assert!(<Vec<Planet> as Rows>::prose_schema(&mut generator).is_none());
    assert_eq!(
        <Prose<Count> as Rows>::prose_schema(&mut generator),
        Some(generator.subschema_for::<Count>())
    );
}

#[test]
fn no_row_has_no_columns_and_a_schema_that_no_value_matches() {
    let schema = SchemaGenerator::default().subschema_for::<NoRow>();

    assert_eq!(NoRow::COLUMNS.len(), 0);
    assert_eq!(NoRow::LONG_COLUMNS.len(), 0);
    assert_eq!(schema.as_bool(), Some(false));
}
