//! Tests for the served failure schemas of a guide that lists field names
//! from another system (ADR-0061).

use serde_json::{Value, json};

use super::{BOOKS_SHOW, tool};
use crate::Envelope;
use crate::help::{Guide, Leaf};
use crate::schemas::documents;
use crate::{Batch, Document, Error};

/// The names that a field of a failure document may have when the guide
/// lists `requestId` and `Content-Type`.
fn widened() -> Value {
    json!({
        "anyOf": [
            { "pattern": "^[a-z0-9_]+$" },
            { "enum": ["requestId", "Content-Type"] },
        ]
    })
}

/// The names that the guides in these tests list.
const NAMES: &[&str] = &["requestId", "Content-Type"];

/// Returns the schema that `guide` serves for the document with `id`.
fn served(guide: &Guide, id: &str) -> Value {
    documents(&tool(), guide)
        .iter()
        .find(|served| served.id() == id)
        .unwrap()
        .schema()
        .to_value()
}

#[test]
fn the_failure_document_accepts_the_field_names_that_the_guide_lists() {
    const LISTING: Guide = Guide::new().field_names(NAMES);

    let schema = served(&LISTING, "shelf/failure/v1");
    let plain = served(&Guide::new(), "shelf/failure/v1");

    assert_eq!(schema["properties"]["fields"]["propertyNames"], widened());
    assert_eq!(
        plain["properties"]["fields"]["propertyNames"],
        json!({ "pattern": "^[a-z0-9_]+$" })
    );
}

#[test]
fn a_batch_s_failure_entries_accept_the_field_names_that_the_guide_lists() {
    const LISTING: Guide = Guide::new()
        .field_names(NAMES)
        .leaves(&[Leaf::new(&["books", "show"]).document(BOOKS_SHOW)]);

    let schema = served(&LISTING, "shelf/books-show/v1");

    assert_eq!(
        schema["properties"]["failures"]["items"]["properties"]["fields"]["propertyNames"],
        widened()
    );
}

#[test]
fn a_batch_in_input_order_accepts_the_field_names_that_the_guide_lists_in_its_failures() {
    const IN_ORDER: Document<Batch<super::Book, Error>> =
        Document::new("shelf", "books-show", 2, "The books for the ids given.")
            .envelope(Envelope::WithoutInOrder);
    const LISTING: Guide = Guide::new()
        .field_names(NAMES)
        .leaves(&[Leaf::new(&["books", "show"]).document(IN_ORDER)]);

    let schema = served(&LISTING, "shelf/books-show/v2");

    assert_eq!(
        schema["items"]["anyOf"][1]["properties"]["fields"]["propertyNames"],
        widened()
    );
}
