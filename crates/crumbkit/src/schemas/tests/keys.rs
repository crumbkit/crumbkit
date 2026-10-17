//! Tests for the schema of a document whose keys the tool declares with
//! `Document::with_keys` (ADR-0059).

use schemars::{Schema, json_schema};
use serde_json::json;

use super::{Book, Shelved, args, output, tool};
use crate::form::{FailureForm, PreparedFailure};
use crate::help::{Guide, Leaf};
use crate::schemas::{SchemasList, document_schema, documents, list};
use crate::{Batch, Document, Error, Listing};

const KEYED: Document<Listing<Book>> =
    Document::new("shelf", "books-list", 4, "One page of the books.")
        .with_keys("$type", "entries")
        .with_type("shelf/get-books/v1");
const KEYED_SUMMARY: Document<Listing<Book, Shelved>> =
    Document::new("shelf", "books-list", 5, "One page of the books.")
        .with_keys("$type", "entries")
        .summary_at_top();
const KEYED_IN_ORDER: Document<Batch<Book, Error>> =
    Document::new("shelf", "books-show", 3, "The books for the ids given.")
        .with_keys("$type", "answers")
        .failures_in_order();

#[test]
fn a_document_with_keys_names_its_type_and_data_under_the_declared_keys() {
    let schema = document_schema(KEYED).to_value();

    assert_eq!(schema["properties"]["$type"]["const"], "shelf/get-books/v1");
    assert_eq!(schema["properties"]["entries"]["type"], "array");
    assert!(schema["properties"].get("schema").is_none());
    assert!(schema["properties"].get("data").is_none());
    assert!(schema["properties"].get("total").is_some());
    assert_eq!(schema["required"], json!(["$type", "entries", "truncated"]));
}

#[test]
fn the_schema_of_a_listing_without_a_total_has_no_total_property() {
    let schema = document_schema(KEYED.without_total()).to_value();

    assert!(schema["properties"].get("total").is_none());
    assert!(schema["properties"].get("truncated").is_some());
    assert_eq!(schema["required"], json!(["$type", "entries", "truncated"]));
}

#[test]
fn a_document_with_keys_and_no_type_value_holds_its_id_under_the_type_key() {
    let schema = document_schema(KEYED_IN_ORDER).to_value();

    assert_eq!(
        schema["properties"]["$type"]["const"],
        "shelf/books-show/v3"
    );
}

#[test]
fn a_batch_in_input_order_with_keys_holds_rows_and_failures_under_its_data_key() {
    let schema = document_schema(KEYED_IN_ORDER).to_value();

    let kinds = schema["properties"]["answers"]["items"]["anyOf"]
        .as_array()
        .unwrap();
    assert_eq!(kinds[0]["$ref"], "#/$defs/Book");
    assert_eq!(
        kinds[1]["properties"]["schema"]["const"],
        "crumbkit/failure/v1"
    );
    assert!(schema["properties"].get("failures").is_none());
    assert_eq!(schema["required"], json!(["$type", "answers"]));
}

#[test]
fn a_summary_at_the_top_puts_the_summary_schema_under_all_of() {
    let schema = document_schema(KEYED_SUMMARY).to_value();

    assert!(schema["properties"].get("summary").is_none());
    assert_eq!(schema["allOf"], json!([{ "$ref": "#/$defs/Shelved" }]));
    assert_eq!(schema["required"], json!(["$type", "entries", "truncated"]));
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
fn a_batch_in_input_order_with_keys_names_the_form_document_schema_for_its_failures() {
    const GUIDE: Guide = Guide::new()
        .leaves(&[Leaf::new(&["books", "show"]).document(KEYED_IN_ORDER)])
        .failure_form(&ErrorForm);

    let served = documents(&tool(), &GUIDE);
    let show = served
        .iter()
        .find(|served| served.id() == "shelf/books-show/v3")
        .unwrap();

    let schema = show.schema().to_value();
    assert_eq!(
        schema["properties"]["answers"]["items"]["anyOf"][1],
        json!({ "type": "object", "required": ["error"] })
    );
}

#[test]
fn list_marks_a_document_with_keys_as_own_envelope() {
    const GUIDE: Guide = Guide::new().leaves(&[
        Leaf::new(&["books", "list"]).document(KEYED),
        Leaf::new(&["books", "show"]).document(KEYED_IN_ORDER),
    ]);

    let listing = list(&mut output(), &GUIDE, &args::<SchemasList>("list", &[])).unwrap();

    let value = serde_json::to_value(&listing).unwrap();
    let kinds: Vec<&str> = value
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["output", "own-envelope", "own-envelope"]);
}
