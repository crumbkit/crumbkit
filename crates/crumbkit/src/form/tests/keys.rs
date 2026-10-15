//! Tests for a batch that prints its answers in input order inside a
//! document with the tool's own envelope keys, with each refusal in the
//! guide's failure form (ADR-0059).

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
