//! Tests for the JSON Schema of a document whose result is bytes.

use serde_json::json;

use super::properties;
use crate::Envelope;
use crate::schemas::document_schema;
use crate::{Bytes, Document};

const FILE: Document<Bytes> = Document::new("files", "file", 1, "The bytes of one file.");

#[test]
fn a_bytes_document_holds_the_bytes_definition_under_data_and_no_failures() {
    let schema = document_schema(FILE).to_value();

    assert_eq!(properties(&schema), ["data", "dry_run", "schema"]);
    assert_eq!(schema["required"], json!(["schema", "data"]));
    assert_eq!(schema["properties"]["data"]["$ref"], "#/$defs/Bytes");
    assert_eq!(
        schema["$defs"]["Bytes"]["oneOf"].as_array().map(Vec::len),
        Some(2)
    );
    assert!(schema["$defs"].get("NoRow").is_none());
}

#[test]
fn an_unwrapped_bytes_document_schema_is_the_bytes_definition() {
    let schema = document_schema(FILE.envelope(Envelope::Without)).to_value();

    assert_eq!(schema["$ref"], "#/$defs/Bytes");
    assert_eq!(schema["title"], "files/file/v1");
}
