//! Tests for the schema of a batch document without the envelope that
//! prints its failures among its rows in input order.

use schemars::{Schema, json_schema};
use serde_json::json;

use super::{Book, tool};
use crate::form::{FailureForm, PreparedFailure};
use crate::help::{Guide, Leaf};
use crate::schemas::{document_schema, documents};
use crate::{Batch, Document, Error};

const IN_ORDER: Document<Batch<Book, Error>> =
    Document::new("shelf", "books-show", 2, "The books for the ids given.")
        .without_envelope()
        .failures_in_order();

#[test]
fn each_item_of_a_batch_in_input_order_is_a_row_or_a_failure_document_with_its_schema() {
    let schema = document_schema(IN_ORDER).to_value();

    assert_eq!(schema["type"], "array");
    let kinds = schema["items"]["anyOf"].as_array().unwrap();
    assert_eq!(kinds[0]["$ref"], "#/$defs/Book");
    assert_eq!(
        kinds[1]["properties"]["schema"]["const"],
        "crumbkit/failure/v1"
    );
    assert_eq!(kinds[1]["properties"]["index"]["minimum"], 0);
}

/// A failure form whose document is `{"error": ...}`.
struct ErrorForm;

impl FailureForm for ErrorForm {
    fn document(&self) -> Document<()> {
        Document::new("shelf", "error", 2, "The document of a failed command.")
    }

    fn schema(&self) -> Schema {
        json_schema!({ "type": "object", "required": ["error"] })
    }

    fn json(&self, failure: &PreparedFailure<'_>) -> serde_json::Value {
        json!({ "error": failure.message() })
    }
}

#[test]
fn a_batch_in_input_order_names_the_form_document_schema_for_its_failures() {
    const GUIDE: Guide = Guide::new()
        .leaves(&[Leaf::new(&["books", "show"]).document(IN_ORDER)])
        .failure_form(&ErrorForm);

    let served = documents(&tool(), &GUIDE);
    let show = served
        .iter()
        .find(|served| served.id() == "shelf/books-show/v2")
        .unwrap();

    let schema = show.schema().to_value();
    assert_eq!(schema["items"]["anyOf"][0]["$ref"], "#/$defs/Book");
    assert_eq!(
        schema["items"]["anyOf"][1],
        json!({ "type": "object", "required": ["error"] })
    );
}
