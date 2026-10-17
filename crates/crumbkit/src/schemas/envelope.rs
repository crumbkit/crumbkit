//! The JSON Schema of a whole document: the object that
//! [`finish`](crate::finish) prints with `--json`, around the rows.

use schemars::generate::SchemaSettings;
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde_json::{Value, json};

use crate::output::failure;
use crate::{Bytes, Document, Rows, Shape};

/// Returns the 2020-12 JSON Schema of the document that
/// [`finish`](crate::finish) prints for `document` with `--json`.
///
/// The schema's `title` is the document's id and its `description` is the
/// document's description. Its properties follow the [`Shape`] of `T`:
///
/// - `schema` holds the id.
/// - `dry_run` is `true` on a dry run, and absent otherwise.
/// - `data` holds the row for [`Shape::Row`], the value for
///   [`Shape::Prose`], the text or base64 object of [`Bytes`] for
///   [`Shape::Bytes`], and an array of rows for every other shape.
/// - `total` holds the number of rows that matched, for [`Shape::Listing`],
///   and is absent when the tool did not count a source with more rows.
/// - `truncated` is true when the document leaves out rows that matched,
///   for [`Shape::Listing`].
/// - `summary` holds the summary, for a listing that has one. A listing
///   without one has [`NoSummary`](crate::NoSummary), whose schema `false`
///   leaves the property out.
/// - `failures` holds the failures of the ids that did not work, for
///   [`Shape::Batch`], and the failures that a prose result leaves out, for
///   [`Shape::Prose`]. Each entry has the fields of `<tool>/failure/v1`
///   without `schema`.
///
/// A document without the envelope has the schema of its `data` at the
/// top.
///
/// The schemas of the row type, the summary type and a prose result's value
/// sit under `$defs`.
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

/// Returns the 2020-12 JSON Schema of one line of the stream that
/// [`stream`](crate::stream) prints for `document` with `--json`.
///
/// Each line is a whole document, so the schema is the one that
/// [`document_schema`] returns for a document of one row of type `R`:
/// `schema` holds the id, `dry_run` is `true` on a dry run, and `data` holds
/// one item. A document without crumbkit's envelope has the schema of one
/// item at the top (ADR-0041). `schemas show` serves it through the leaf,
/// which holds the document without its row type, so only the tests call
/// this function.
#[cfg(test)]
#[must_use]
pub(crate) fn stream_schema<R: crate::Row>(document: Document<crate::Stream<R>>) -> Schema {
    envelope::<R>(document.untyped())
}

/// Returns the schema that [`document_schema`] describes for the document
/// `document` of the result type `T`. A help leaf keeps a pointer to it,
/// because the leaf holds its document without the result type.
///
/// A document without crumbkit's envelope has the schema of its `data` at
/// the top, with the document's `title` and `description`, because
/// `finish` prints the result alone (ADR-0007).
pub(crate) fn envelope<T: Rows>(document: Document<()>) -> Schema {
    let settings = SchemaSettings::draft2020_12();
    let meta_schema = settings.meta_schema.clone();
    let mut generator = settings.into_generator();
    let data = match T::prose_schema(&mut generator) {
        Some(value) => value,
        None if T::SHAPE == Shape::Row => generator.subschema_for::<T::Row>(),
        None if T::SHAPE == Shape::Bytes => generator.subschema_for::<Bytes>(),
        None => json_schema!({
            "description": data_description(T::SHAPE),
            "type": "array",
            "items": generator.subschema_for::<T::Row>(),
        }),
    };
    if !document.get_envelope().wraps() {
        return top(document, data, meta_schema.as_deref(), &mut generator);
    }
    let mut properties = serde_json::Map::new();
    let mut required = vec!["schema", "data"];
    properties.insert(
        "schema".to_owned(),
        json!({
            "description": "The id of this document.",
            "const": document.to_string(),
        }),
    );
    properties.insert(
        "dry_run".to_owned(),
        json_schema!({
            "description": "Present, and true, when the run was a dry run that changed nothing.",
            "const": true,
        })
        .to_value(),
    );
    properties.insert("data".to_owned(), data.to_value());
    if T::SHAPE == Shape::Listing {
        properties.insert(
            "total".to_owned(),
            json!({
                "description": "The number of rows that matched, of which data holds one page. It is absent when the tool did not count a source that holds more rows.",
                "type": "integer",
                "minimum": 0,
            }),
        );
    }
    if T::SHAPE == Shape::Listing {
        properties.insert(
            "truncated".to_owned(),
            json!({
                "description": "True when the document leaves out rows that matched, because of the limit or the output budget, and false when data holds every row.",
                "type": "boolean",
            }),
        );
        required.push("truncated");
        let summary = generator.subschema_for::<T::Summary>();
        // A listing without a summary has no summary property.
        if summary.as_bool() != Some(false) {
            properties.insert("summary".to_owned(), summary.to_value());
            required.push("summary");
        }
    }
    if T::SHAPE == Shape::Listing || T::SHAPE == Shape::List {
        properties.insert("cut".to_owned(), cut_schema());
    }
    if T::SHAPE == Shape::Batch || T::SHAPE == Shape::Prose {
        properties.insert(
            "failures".to_owned(),
            json!({
                "description": failures_description(T::SHAPE),
                "type": "array",
                "items": failure::entry_schema(),
            }),
        );
        required.push("failures");
    }
    let body = json_schema!({
        "type": "object",
        "properties": properties,
        "required": required,
    });
    top(document, body, meta_schema.as_deref(), &mut generator)
}

/// Returns the schema of `cut`, which a listing's or a list's document holds
/// when the output budget cut it short (ADR-0046).
fn cut_schema() -> Value {
    json!({
        "description": "Present when the output budget cut the result short, so data holds only its first rows.",
        "type": "object",
        "properties": {
            "code": {
                "description": "The over-budget code, in the tool's spelling.",
                "type": "string",
            },
            "budget": {
                "description": "The most bytes that the run printed on standard output.",
                "type": "integer",
                "minimum": 0,
            },
        },
        "required": ["code", "budget"],
        "additionalProperties": false,
    })
}

/// Returns the JSON Schema of the file format `document` whose files hold a
/// `T`: the schema of `T` with the format's `title` and `description`, as
/// [`FileFormat`](crate::help::FileFormat) describes.
pub(crate) fn file_format<T: JsonSchema>(document: Document<()>) -> Schema {
    let settings = SchemaSettings::draft2020_12();
    let meta_schema = settings.meta_schema.clone();
    let mut generator = settings.into_generator();
    let body = generator.subschema_for::<T>();
    top(document, body, meta_schema.as_deref(), &mut generator)
}

/// Returns the top of a document's schema: the document's id as `title`
/// and its description, then the other keywords of `body`, `$schema` when `meta_schema` is set, and the definitions
/// that `generator` collected under `$defs`.
fn top(
    document: Document<()>,
    body: Schema,
    meta_schema: Option<&str>,
    generator: &mut SchemaGenerator,
) -> Schema {
    let mut schema = json_schema!({
        "title": document.to_string(),
        "description": document.description(),
    });
    match body.as_object() {
        Some(keywords) => {
            for (keyword, value) in keywords {
                if keyword != "title" && keyword != "description" {
                    schema.insert(keyword.clone(), value.clone());
                }
            }
        }
        // A boolean schema has no keywords, so the top holds it under
        // `allOf`. No result type of crumbkit's has one at the top.
        None => {
            schema.insert("allOf".to_owned(), json!([body.to_value()]));
        }
    }
    if let Some(meta_schema) = meta_schema {
        schema.insert("$schema".to_owned(), Value::from(meta_schema));
    }
    let definitions = generator.take_definitions(true);
    if !definitions.is_empty() {
        schema.insert("$defs".to_owned(), Value::Object(definitions));
    }
    schema
}

/// Returns the description of the `failures` array of a document of
/// `shape`, which is a batch or a prose result.
fn failures_description(shape: Shape) -> &'static str {
    match shape {
        Shape::Prose => "The failures that the answer leaves out. It is empty when nothing failed.",
        _ => "The failures of the ids that did not work. It is empty when every id worked.",
    }
}

/// Returns the description of the `data` array of a document of `shape`.
fn data_description(shape: Shape) -> &'static str {
    match shape {
        Shape::Listing => "The rows of the page, up to the limit.",
        Shape::Batch => "The rows for the ids that worked, in the order the caller gave the ids.",
        _ => "The rows, in order.",
    }
}
