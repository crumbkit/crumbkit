//! Tests for a batch that prints its answers in input order inside a
//! document with the tool's own envelope keys, with each refusal in the
//! guide's failure form (ADR-0059).

use std::io::{self, Write};
use std::process::ExitCode;

use schemars::Schema;
use serde_json::{Value, json};

use super::{ErrorForm, Missing, PLANETS_SHOW, Planet, finish};
use crate::form::{FailureForm, PreparedFailure};
use crate::help::Guide;
use crate::{Batch, Document};

/// A form whose document is `{"$type": "error", "key": ..., "code": ...}`.
struct TypedForm;

impl FailureForm for TypedForm {
    fn document(&self) -> Document<()> {
        ErrorForm.document()
    }

    fn schema(&self) -> Schema {
        ErrorForm.schema()
    }

    fn json(&self, failure: &PreparedFailure<'_>) -> Value {
        json!({ "$type": "error", "key": failure.key(), "code": failure.code() })
    }
}

const TYPED_GUIDE: Guide = Guide::new().failure_form(&TypedForm);

/// A form whose text names the key only when the batch has several keys.
struct KeyedTextForm;

impl FailureForm for KeyedTextForm {
    fn document(&self) -> Document<()> {
        ErrorForm.document()
    }

    fn schema(&self) -> Schema {
        ErrorForm.schema()
    }

    fn json(&self, failure: &PreparedFailure<'_>) -> Value {
        json!({ "code": failure.code() })
    }

    fn write_text(&self, err: &mut dyn Write, failure: &PreparedFailure<'_>) -> io::Result<()> {
        match failure.key().filter(|_| failure.among_several()) {
            Some(key) => writeln!(err, "{key}: {}", failure.message()),
            None => writeln!(err, "{}", failure.message()),
        }
    }
}

const KEYED_TEXT_GUIDE: Guide = Guide::new().failure_form(&KeyedTextForm);

#[test]
fn a_form_learns_that_a_failure_is_among_several_keys_only_when_the_batch_has_more_than_one() {
    let mut one: Batch<Planet, Missing> = Batch::new(&["show"], "names", &["Vulcan"]);
    one.push(Err(Missing));
    let mut two: Batch<Planet, Missing> = Batch::new(&["show"], "names", &["Mars", "Vulcan"]);
    two.push(Ok(Planet { name: "Mars" }));
    two.push(Err(Missing));

    let alone = finish(&KEYED_TEXT_GUIDE, &["list"], PLANETS_SHOW, Ok(one));
    let among = finish(&KEYED_TEXT_GUIDE, &["list"], PLANETS_SHOW, Ok(two));

    assert_eq!(alone.err, "the solar system has no planet Vulcan\n");
    assert_eq!(among.err, "Vulcan: the solar system has no planet Vulcan\n");
}

#[test]
fn a_batch_in_input_order_prints_one_answer_per_key_asked_under_its_own_keys_with_each_refusal_in_the_form()
 {
    let document = PLANETS_SHOW
        .with_keys("$type", "answers")
        .with_type("tool/get-report/v1")
        .failures_in_order();
    let mut batch: Batch<Planet, Missing> =
        Batch::new(&["show"], "names", &["Mars", "Vulcan", "Mars", "Vulcan"]);
    batch.push(Ok(Planet { name: "Mars" }));
    batch.push(Err(Missing));
    batch.push(Ok(Planet { name: "Mars" }));
    batch.push(Err(Missing));

    let ran = finish(&TYPED_GUIDE, &["list", "--json"], document, Ok(batch));

    assert_eq!(ran.code, ExitCode::FAILURE);
    assert_eq!(ran.err, "");
    let printed: Value = serde_json::from_str(&ran.out).unwrap();
    assert_eq!(
        printed,
        json!({
            "$type": "tool/get-report/v1",
            "answers": [
                { "name": "Mars" },
                { "$type": "error", "key": "Vulcan", "code": "solar.planets.not_found" },
                { "name": "Mars" },
                { "$type": "error", "key": "Vulcan", "code": "solar.planets.not_found" },
            ],
        })
    );
}
