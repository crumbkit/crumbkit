//! Tests for the documents of stream commands, which `schemas list` marks
//! with the form `stream`.

use serde_json::json;

use super::{Book, args, output, tool};
use crate::help::{Guide, Leaf};
use crate::schemas::{SchemasList, documents, list, stream_schema};
use crate::{Document, Stream};

const BOOKS_WATCH: Document<Stream<Book>> =
    Document::new("shelf", "books-watch", 1, "Each book as it is shelved.");

/// The test guide, with `books list` printing a stream.
const STREAMS: Guide = Guide::new().leaves(&[
    Leaf::new(&["books", "list"]).stream(BOOKS_WATCH),
    Leaf::new(&["schemas", "list"]).document(crate::schemas::SCHEMAS_LIST),
]);

#[test]
fn a_stream_document_schema_describes_one_line_with_one_item_under_data() {
    let schema = stream_schema(BOOKS_WATCH).to_value();

    assert_eq!(schema["title"], "shelf/books-watch/v1");
    assert_eq!(schema["description"], "Each book as it is shelved.");
    assert_eq!(schema["required"], json!(["schema", "data"]));
    assert_eq!(schema["properties"]["data"]["$ref"], "#/$defs/Book");
}

#[test]
fn a_stream_leaf_serves_its_document_with_the_leaf_s_command_line() {
    let served = documents(&tool(), &STREAMS);

    assert_eq!(served[1].id(), "shelf/books-watch/v1");
    assert_eq!(served[1].commands(), ["shelf books list"]);
    assert_eq!(served[1].schema(), stream_schema(BOOKS_WATCH));
}

#[test]
fn list_marks_each_document_with_its_form() {
    let listing = list(&mut output(), &STREAMS, &args::<SchemasList>("list", &[])).unwrap();

    let value = serde_json::to_value(&listing).unwrap();
    let forms: Vec<&str> = value
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["form"].as_str().unwrap())
        .collect();
    assert_eq!(forms, ["document", "stream", "document"]);
}

#[test]
fn a_leaf_reports_whether_it_declares_a_stream() {
    let [stream, document] = STREAMS.get_leaves() else {
        panic!("the guide has two leaves");
    };

    assert!(stream.is_stream());
    assert!(!document.is_stream());
    assert_eq!(
        stream.get_document().map(|document| document.to_string()),
        Some("shelf/books-watch/v1".to_owned())
    );
}
