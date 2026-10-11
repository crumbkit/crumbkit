//! The JSON Schema of a whole document: the object that
//! [`finish`](crate::finish) prints with `--json`, around the rows.

use schemars::generate::SchemaSettings;
use schemars::{Schema, json_schema};
use serde_json::{Value, json};

use crate::output::failure;
use crate::{Document, Rows, Shape};

/// Returns the 2020-12 JSON Schema of the document that
/// [`finish`](crate::finish) prints for `document` with `--json`.
///
/// The schema's `title` is the document's id and its `description` is the
/// document's description. Its properties follow the [`Shape`] of `T`:
///
/// - `schema` holds the id.
/// - `data` holds the row for [`Shape::Row`], and an array of rows for
///   every other shape.
/// - `total` holds the number of rows that matched, for [`Shape::Listing`].
/// - `summary` holds the summary, for a listing that has one. A listing
///   without one has [`NoSummary`](crate::NoSummary), whose schema `false`
///   leaves the property out.
/// - `failures` holds the failures of the ids that did not work, for
///   [`Shape::Batch`]. Each entry has the fields of `crumbkit/failure/v1`
///   without `schema`.
///
/// The schemas of the row type and the summary type sit under `$defs`.
/// `schemas show` serves this schema, and crumbkit's checked-in schema
/// files hold it, so the two cannot differ (ADR-0007).
///
/// ```
/// # #[derive(serde::Serialize, schemars::JsonSchema)]
/// # struct Book { id: u32 }
/// # impl crumbkit::Row for Book { const COLUMNS: &'static [&'static str] = &["id"]; }
/// use crumbkit::{Document, Listing};
///
/// const BOOKS_LIST: Document<Listing<Book>> =
///     Document::new("shelf", "books-list", 1, "One page of the books.");
///
/// let schema = crumbkit::schemas::document_schema(BOOKS_LIST);
/// assert_eq!(schema.get("title").unwrap(), "shelf/books-list/v1");
/// ```
#[must_use]
pub fn document_schema<T: Rows>(document: Document<T>) -> Schema {
    envelope::<T>(document.untyped())
}

/// Returns the schema that [`document_schema`] describes for the document
/// `document` of the result type `T`. A help leaf keeps a pointer to it,
/// because the leaf holds its document without the result type.
pub(crate) fn envelope<T: Rows>(document: Document<()>) -> Schema {
    let settings = SchemaSettings::draft2020_12();
    let meta_schema = settings.meta_schema.clone();
    let mut generator = settings.into_generator();
    let row = generator.subschema_for::<T::Row>();
    let data = if T::SHAPE == Shape::Row {
        row
    } else {
        json_schema!({
            "description": data_description(T::SHAPE),
            "type": "array",
            "items": row,
        })
    };
    let mut properties = serde_json::Map::new();
    let mut required = vec!["schema", "data"];
    properties.insert(
        "schema".to_owned(),
        json_schema!({
            "description": "The id of this document.",
            "const": document.to_string(),
        })
        .to_value(),
    );
    properties.insert("data".to_owned(), data.to_value());
    if T::SHAPE == Shape::Listing {
        properties.insert(
            "total".to_owned(),
            json!({
                "description": "The number of rows that matched, of which data holds one page.",
                "type": "integer",
                "minimum": 0,
            }),
        );
        required.push("total");
        let summary = generator.subschema_for::<T::Summary>();
        if summary.as_bool() != Some(false) {
            properties.insert("summary".to_owned(), summary.to_value());
            required.push("summary");
        }
    }
    if T::SHAPE == Shape::Batch {
        properties.insert(
            "failures".to_owned(),
            json!({
                "description": "The failures of the ids that did not work. It is empty when every id worked.",
                "type": "array",
                "items": failure::entry_schema(),
            }),
        );
        required.push("failures");
    }
    let mut schema = json_schema!({
        "title": document.to_string(),
        "description": document.description(),
        "type": "object",
        "properties": properties,
        "required": required,
    });
    if let Some(meta_schema) = meta_schema {
        schema.insert("$schema".to_owned(), Value::from(meta_schema.as_ref()));
    }
    let definitions = generator.take_definitions(true);
    if !definitions.is_empty() {
        schema.insert("$defs".to_owned(), Value::Object(definitions));
    }
    schema
}

/// Returns the description of the `data` array of a document of `shape`.
fn data_description(shape: Shape) -> &'static str {
    match shape {
        Shape::Listing => "The rows of the page, up to the limit.",
        Shape::Batch => "The rows for the ids that worked, in the order the caller gave the ids.",
        _ => "The rows, in order.",
    }
}
