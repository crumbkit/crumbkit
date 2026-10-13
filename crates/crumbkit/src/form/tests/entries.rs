//! Tests for the entries of a batch's or a prose result's `failures`,
//! which a failure form writes in its own shape.

use std::fmt;
use std::process::ExitCode;

use schemars::{JsonSchema, Schema, json_schema};
use serde::Serialize;
use serde_json::{Value, json};

use super::{ErrorForm, Missing, PLANETS_SHOW, Planet, finish};
use crate::form::{FailureForm, PreparedFailure};
use crate::help::Guide;
use crate::{Batch, Document, Prose};

/// A form whose entries are `{"error": {"code": ..., "message": ...}}`.
struct EntryForm;

impl FailureForm for EntryForm {
    fn document(&self) -> Document<()> {
        ErrorForm.document()
    }

    fn schema(&self) -> Schema {
        ErrorForm.schema()
    }

    fn json(&self, failure: &PreparedFailure<'_>) -> Value {
        ErrorForm.json(failure)
    }

    fn entry(&self, failure: &PreparedFailure<'_>) -> Value {
        json!({ "error": { "code": failure.code(), "message": failure.message() } })
    }

    fn entry_schema(&self) -> Schema {
        json_schema!({ "type": "object", "required": ["error"] })
    }
}

const ENTRY_GUIDE: Guide = Guide::new().failure_form(&EntryForm);

/// The value of a prose result of the test tool.
#[derive(Serialize, JsonSchema)]
struct Count {
    planets: u32,
}

impl fmt::Display for Count {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} planets", self.planets)
    }
}

const PLANETS_COUNT: Document<Prose<Count, Missing>> =
    Document::new("solar", "planets-count", 1, "The test prose.");

/// The entry that `EntryForm` writes for a `Missing` spelled with dots.
fn missing_entry() -> Value {
    json!({ "error": {
        "code": "solar.planets.not_found",
        "message": "the solar system has no planet Vulcan",
    }})
}

#[test]
fn a_batch_entry_is_written_by_the_guide_failure_form() {
    let batch = Batch::new(vec![Planet { name: "Mars" }], vec![Missing]);
    let ran = finish(&ENTRY_GUIDE, &["list", "--json"], PLANETS_SHOW, Ok(batch));

    assert_eq!(ran.code, ExitCode::FAILURE);
    let document: Value = serde_json::from_str(&ran.out).unwrap();
    assert_eq!(document["schema"], "solar/planets-show/v1");
    assert_eq!(document["failures"], json!([missing_entry()]));
}

#[test]
fn a_prose_result_entry_is_written_by_the_guide_failure_form() {
    let prose = Prose::with_failures(Count { planets: 8 }, vec![Missing]);
    let ran = finish(&ENTRY_GUIDE, &["list", "--json"], PLANETS_COUNT, Ok(prose));

    assert_eq!(ran.code, ExitCode::FAILURE);
    let document: Value = serde_json::from_str(&ran.out).unwrap();
    assert_eq!(document["data"], json!({ "planets": 8 }));
    assert_eq!(document["failures"], json!([missing_entry()]));
}

#[test]
fn the_failure_of_the_ids_that_the_budget_leaves_out_is_written_by_the_form_entry() {
    const BUDGET_GUIDE: Guide = ENTRY_GUIDE.output_budget(1_024, "the tests print little");
    let rows = (0..200).map(|_| Planet { name: "Mars" }).collect();
    let batch = Batch::new(rows, vec![Missing]);
    let ran = finish(&BUDGET_GUIDE, &["list", "--json"], PLANETS_SHOW, Ok(batch));

    assert_eq!(ran.code, ExitCode::FAILURE);
    assert!(ran.out.len() <= 1_024, "{}", ran.out.len());
    let document: Value = serde_json::from_str(&ran.out).unwrap();
    let failures = document["failures"].as_array().unwrap();
    assert_eq!(failures.len(), 2);
    assert_eq!(failures[0], missing_entry());
    assert_eq!(failures[1]["error"]["code"], "crumbkit.output.over_budget");
}
