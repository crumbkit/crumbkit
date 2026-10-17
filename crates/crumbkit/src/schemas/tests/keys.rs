//! Tests for the schema of a document whose envelope declares its own keys
//! (ADR-0059).

use schemars::{Schema, json_schema};
use serde_json::json;

use super::{Book, args, output, tool};
use crate::Envelope;
use crate::form::{FailureForm, PreparedFailure};
use crate::help::{Guide, Leaf};
use crate::schemas::{SchemasList, document_schema, documents, list};
use crate::{Batch, Document, Error, Listing};

const KEYED: Document<Listing<Book>> =
    Document::new("shelf", "books-list", 4, "One page of the books.").envelope(Envelope::Keys {
        type_key: "$type",
        data_key: "entries",
    });
const KEYED_IN_ORDER: Document<Batch<Book, Error>> =
    Document::new("shelf", "books-show", 3, "The books for the ids given.").envelope(
        Envelope::KeysInOrder {
            type_key: "$type",
            data_key: "answers",
        },
    );

#[test]
fn a_document_with_keys_holds_its_id_and_data_under_the_declared_keys() {
    let schema = document_schema(KEYED).to_value();

    assert_eq!(
        schema["properties"]["$type"]["const"],
        "shelf/books-list/v4"
    );
    assert_eq!(schema["properties"]["entries"]["type"], "array");
    assert!(schema["properties"].get("schema").is_none());
    assert!(schema["properties"].get("data").is_none());
    assert!(schema["properties"].get("total").is_some());
    assert_eq!(schema["required"], json!(["$type", "entries", "truncated"]));
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
