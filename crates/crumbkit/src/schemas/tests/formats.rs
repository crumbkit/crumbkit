//! Tests for the documents that `schemas list` serves beside the documents
//! that commands print in crumbkit's envelope: documents without the
//! envelope and file formats that no command prints.

use schemars::{JsonSchema, Schema, json_schema};
use serde::Serialize;
use serde_json::json;

use super::{Book, Shelved, args, output, tool};
use crate::Envelope;
use crate::form::{FailureForm, PreparedFailure};
use crate::help::{FileFormat, Guide, Leaf};
use crate::schemas::{SchemasList, SchemasShow, document_schema, documents, list, show};
use crate::{Document, Listing, Prose};

/// The settings file of the test tool, which users write.
#[derive(Serialize, JsonSchema)]
struct Settings {
    /// The number of books that a page shows.
    page: u32,
}

const SETTINGS: Document<Settings> = Document::new(
    "shelf",
    "settings",
    1,
    "The settings file that users write.",
);
const LEGACY: Document<Listing<Book>> =
    Document::new("shelf", "books-list", 3, "The books as scripts read them.")
        .envelope(Envelope::Without);

/// The test guide's leaves, with `books list` printing [`LEGACY`], and the
/// settings file registered as a file format.
const FORMATS: Guide = Guide::new()
    .leaves(&[
        Leaf::new(&["books", "list"]).document(LEGACY),
        Leaf::new(&["schemas", "list"]).document(crate::schemas::SCHEMAS_LIST),
    ])
    .documents(&[FileFormat::new(SETTINGS)]);

#[test]
fn an_unwrapped_document_schema_is_the_schema_of_data_at_the_top() {
    let schema = document_schema(LEGACY).to_value();

    assert_eq!(
        schema["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    assert_eq!(schema["title"], "shelf/books-list/v3");
    assert_eq!(schema["description"], "The books as scripts read them.");
    assert_eq!(schema["type"], "array");
    assert_eq!(schema["items"]["$ref"], "#/$defs/Book");
    assert!(schema.get("properties").is_none());
}

#[test]
fn an_unwrapped_prose_document_schema_is_the_schema_of_its_value() {
    const SUMMARY: Document<Prose<Shelved>> =
        Document::new("shelf", "shelf-summary", 1, "What the shelf holds.")
            .envelope(Envelope::Without);

    let schema = document_schema(SUMMARY).to_value();

    assert_eq!(schema["$ref"], "#/$defs/Shelved");
    assert_eq!(schema["title"], "shelf/shelf-summary/v1");
    assert_eq!(schema["description"], "What the shelf holds.");
}

#[test]
fn a_file_format_schema_is_the_schema_of_its_type_with_the_id_as_its_title() {
    let schema = FileFormat::new(SETTINGS).schema().to_value();

    assert_eq!(schema["title"], "shelf/settings/v1");
    assert_eq!(schema["description"], "The settings file that users write.");
    assert_eq!(
        schema["$defs"]["Settings"]["properties"]["page"]["description"],
        "The number of books that a page shows."
    );
}

#[test]
fn file_formats_come_after_the_leaf_documents_and_name_no_command() {
    let served = documents(&tool(), &FORMATS);

    let ids: Vec<String> = served.iter().map(super::super::Served::id).collect();
    assert_eq!(
        ids,
        [
            "crumbkit/failure/v1",
            "shelf/books-list/v3",
            "crumbkit/schemas-list/v1",
            "shelf/settings/v1",
        ]
    );
    assert_eq!(served[3].commands(), [] as [String; 0]);
}

#[test]
fn list_marks_each_document_as_output_unwrapped_or_file() {
    let listing = list(&mut output(), &FORMATS, &args::<SchemasList>("list", &[])).unwrap();

    let value = serde_json::to_value(&listing).unwrap();
    let kinds: Vec<&str> = value
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["output", "unwrapped", "output", "file"]);
    assert_eq!(
        value[3],
        json!({
            "id": "shelf/settings/v1",
            "kind": "file",
            "form": "document",
            "description": "The settings file that users write.",
            "commands": [],
        })
    );
}

#[test]
fn show_serves_the_schema_of_a_file_format() {
    let batch = show(
        &mut output(),
        &FORMATS,
        &args::<SchemasShow>("show", &["shelf/settings/v1"]),
    )
    .unwrap();

    let value = serde_json::to_value(&batch).unwrap();
    assert_eq!(
        value[0]["schema"],
        FileFormat::new(SETTINGS).schema().to_value()
    );
}

/// A tool's own failure form, whose document is `shelf/error/v2`.
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
fn a_failure_form_document_replaces_the_failure_document_as_unwrapped_with_no_command() {
    const GUIDE: Guide = FORMATS.failure_form(&ErrorForm);

    let listing = list(&mut output(), &GUIDE, &args::<SchemasList>("list", &[])).unwrap();
    let batch = show(
        &mut output(),
        &GUIDE,
        &args::<SchemasShow>("show", &["shelf/error/v2"]),
    )
    .unwrap();

    let value = serde_json::to_value(&listing).unwrap();
    assert_eq!(value.as_array().unwrap().len(), 4);
    assert_eq!(
        value[0],
        json!({
            "id": "shelf/error/v2",
            "kind": "unwrapped",
            "form": "document",
            "description": "The document of a failed command.",
            "commands": [],
        })
    );
    let shown = serde_json::to_value(&batch).unwrap();
    assert_eq!(
        shown[0]["schema"],
        json!({ "type": "object", "required": ["error"], "title": "shelf/error/v2" })
    );
}

/// A failure form whose batch entries are `{"error": ...}` objects.
struct EntryForm;

impl FailureForm for EntryForm {
    fn document(&self) -> Document<()> {
        ErrorForm.document()
    }

    fn schema(&self) -> Schema {
        ErrorForm.schema()
    }

    fn json(&self, failure: &PreparedFailure<'_>) -> serde_json::Value {
        ErrorForm.json(failure)
    }

    fn entry_schema(&self) -> Schema {
        json_schema!({ "type": "object", "required": ["error"] })
    }
}

/// Returns the schema that `guide` serves for `shelf/books-show/v1`.
fn books_show_schema(guide: &Guide) -> serde_json::Value {
    documents(&tool(), guide)
        .iter()
        .find(|served| served.id() == "shelf/books-show/v1")
        .unwrap()
        .schema()
        .to_value()
}

#[test]
fn the_batch_schema_names_the_form_entry_schema_for_its_failures() {
    const GUIDE: Guide = super::GUIDE.failure_form(&EntryForm);

    let schema = books_show_schema(&GUIDE);

    assert_eq!(
        schema["properties"]["failures"]["items"],
        json!({ "type": "object", "required": ["error"] })
    );
}

#[test]
fn a_form_without_its_own_entry_serves_the_batch_schema_of_crumbkit_entries() {
    const GUIDE: Guide = super::GUIDE.failure_form(&ErrorForm);

    assert_eq!(
        books_show_schema(&GUIDE),
        document_schema(super::BOOKS_SHOW).to_value()
    );
}
