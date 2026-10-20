//! The JSON Schema of the `crumbkit/failure/v1` document and of an entry of
//! a batch's `failures`, written by hand so crumbkit needs no schemars
//! `derive` feature.

use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};

use super::{FAILURE, PreparedFailure};

/// Describes the `crumbkit/failure/v1` document as a 2020-12 JSON Schema.
///
/// This impl is written by hand so crumbkit needs no schemars `derive`
/// feature. `crates/crumbkit/schemas/failure-v1.json` holds the schema, and a
/// test fails when the two differ (ADR-0007).
impl JsonSchema for PreparedFailure<'_> {
    fn schema_name() -> Cow<'static, str> {
        "FailureDocument".into()
    }

    fn schema_id() -> Cow<'static, str> {
        "crumbkit/failure/v1".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "description": "The document that a command prints on standard output when it fails with --json.",
            "type": "object",
            "properties": {
                "schema": {
                    "description": "The id of this document.",
                    "const": "crumbkit/failure/v1",
                },
                "key": {
                    "description": "The key that the caller gave for the answer that failed, when the failure is one answer of a batch.",
                    "type": "string",
                },
                "index": {
                    "description": "The position of key among the keys that the caller gave, counted from 0, so two failures of the same key differ. It is present only with key.",
                    "type": "integer",
                    "minimum": 0,
                },
                "code": {
                    "description": "The failure's stable code in the tool's spelling, such as shelf::books::not_found. It is one or more printable ASCII characters without spaces, from ! to ~.",
                    "type": "string",
                    "pattern": "^[!-~]+$",
                },
                "message": {
                    "description": "What went wrong.",
                    "type": "string",
                },
                "cause": {
                    "description": "The errors that led to the failure, joined with \": \". It is present only when the failure has a cause.",
                    "type": "string",
                },
                "fields": {
                    "description": "The values at fault, in the order the failure gives them, each under a name of one or more lowercase ASCII letters, digits and underscores. A value is a string, an integer, a boolean, null, or an array or object of such values, and an object holds each key once. It is present only when the failure names at least one.",
                    "type": "object",
                    "propertyNames": { "pattern": "^[a-z0-9_]+$" },
                    "additionalProperties": field_value_schema(),
                },
                "labels": labels_schema(),
                "related": related_schema(),
                "recovery": step_schema("What the caller can do next, tagged by kind."),
                "alternatives": {
                    "description": "Other repairs the caller can try, in order of preference, each in the shape of recovery. It is present only when the failure offers at least one.",
                    "type": "array",
                    "items": step_schema("A repair the caller can try instead, tagged by kind."),
                },
                "url": {
                    "description": "A page that explains the failure. It is present only when the failure has one.",
                    "type": "string",
                },
            },
            "required": ["schema", "code", "message", "recovery"],
        })
    }
}

/// Returns the JSON Schema of the `crumbkit/failure/v1` document, with the
/// document's id as its `title`, as `schemas show` serves it and
/// `crates/crumbkit/schemas/failure-v1.json` holds it.
pub(crate) fn document_schema() -> Schema {
    let mut schema = crate::schemas::settings()
        .into_generator()
        .into_root_schema_for::<PreparedFailure<'static>>();
    schema.insert("title".to_owned(), FAILURE.to_string().into());
    schema
}

/// Returns the JSON Schema of the `crumbkit/failure/v1` document without
/// its `title` and `$schema`, which [`entry_schema`] builds on.
pub(super) fn failure_schema() -> Schema {
    <PreparedFailure<'static>>::json_schema(&mut crate::schemas::settings().into_generator())
}

/// Returns the JSON Schema of one entry of a batch's `failures` array: the
/// schema of the `crumbkit/failure/v1` document without its `schema`
/// property, as [`Entry`] writes it.
pub(crate) fn entry_schema() -> Schema {
    let mut schema = failure_schema();
    schema.insert(
        "description".to_owned(),
        "A failure of some of the ids: the fields of the failure document without schema.".into(),
    );
    if let Some(properties) = schema
        .get_mut("properties")
        .and_then(serde_json::Value::as_object_mut)
    {
        properties.remove("schema");
    }
    if let Some(required) = schema
        .get_mut("required")
        .and_then(serde_json::Value::as_array_mut)
    {
        required.retain(|name| name != "schema");
    }
    schema
}

/// Describes one step of the `crumbkit/failure/v1` document, tagged by
/// `kind`, as `recovery` and each entry of `alternatives` hold it, with
/// `description` as its description.
fn step_schema(description: &str) -> Schema {
    json_schema!({
        "description": description,
        "oneOf": [
            {
                "description": "Run this command. With `all`, run it once with every value of the failure's field that `all` names in place of the argument's placeholder, such as `<ID>...`.",
                "type": "object",
                "properties": {
                    "kind": { "const": "run" },
                    "call": { "type": "string" },
                    "all": { "type": "string" },
                },
                "required": ["kind", "call"],
            },
            {
                "description": "Run the same command again later.",
                "type": "object",
                "properties": {
                    "kind": { "const": "retry" },
                },
                "required": ["kind"],
            },
            {
                "description": "Follow this advice.",
                "type": "object",
                "properties": {
                    "kind": { "const": "hint" },
                    "text": { "type": "string" },
                },
                "required": ["kind", "text"],
            },
            {
                "description": "There is no next step, for this reason.",
                "type": "object",
                "properties": {
                    "kind": { "const": "none" },
                    "reason": { "type": "string" },
                },
                "required": ["kind", "reason"],
            },
        ],
    })
}

/// Describes one value under `fields`: a string, an integer, a boolean,
/// null, or an array or object whose values have this schema too. The
/// schema has the `$anchor` `field_value`, and its `items` and
/// `additionalProperties` refer to it, because a value nests, as
/// `related_schema` does for related diagnostics. `integer` admits no
/// float at any depth (ADR-0007).
fn field_value_schema() -> Schema {
    json_schema!({
        "$anchor": "field_value",
        "type": ["string", "integer", "boolean", "null", "array", "object"],
        "items": { "$ref": "#field_value" },
        "additionalProperties": { "$ref": "#field_value" },
    })
}

/// Describes the `labels` array of the `crumbkit/failure/v1` document. The
/// schema lists it whether or not the `miette` feature is on, so there is
/// one checked-in schema file, and readers ignore a field they do not know
/// (ADR-0007).
fn labels_schema() -> Schema {
    json_schema!({
        "description": "The places in a source, such as a file, that the failure points at, in the order the failure gives them. It is present only when the failure has at least one label.",
        "type": "array",
        "items": {
            "type": "object",
            "properties": {
                "offset": {
                    "description": "The byte offset of the start of the span from the start of the source.",
                    "type": "integer",
                    "minimum": 0,
                },
                "length": {
                    "description": "The length of the span in bytes.",
                    "type": "integer",
                    "minimum": 0,
                },
                "name": {
                    "description": "The name of the source, such as a file name. It is present only when the span could be read and the source has a name.",
                    "type": "string",
                },
                "line": {
                    "description": "The 1-based line that holds the start of the span. It is present only when the span could be read.",
                    "type": "integer",
                    "minimum": 1,
                },
                "column": {
                    "description": "The 1-based column of the start of the span, counted in characters of its line. It is present only when the span could be read.",
                    "type": "integer",
                    "minimum": 1,
                },
                "excerpt": {
                    "description": "The lines of the source that the span covers, with one line before and one after where the source has them. It is present only when the span could be read.",
                    "type": "object",
                    "properties": {
                        "line": {
                            "description": "The 1-based number of the first line of the excerpt.",
                            "type": "integer",
                            "minimum": 1,
                        },
                        "text": {
                            "description": "The lines, joined by the line breaks of the source, without a line break at the end.",
                            "type": "string",
                        },
                    },
                    "required": ["line", "text"],
                },
                "text": {
                    "description": "What the label says about the span. It is present only when the label has text.",
                    "type": "string",
                },
            },
            "required": ["offset", "length"],
        },
    })
}

/// Describes the `related` array of the `crumbkit/failure/v1` document. Each
/// entry has the `$anchor` `related`, and its own `related` array refers to
/// the entry's schema through it, because related diagnostics nest. The
/// schema lists the array whether or not the `miette` feature is on, as it
/// does `labels`.
fn related_schema() -> Schema {
    json_schema!({
        "description": "The diagnostics that the failure's diagnostic relates to it, in the order it gives them. It is present only when there is at least one.",
        "type": "array",
        "items": {
            "$anchor": "related",
            "type": "object",
            "properties": {
                "message": {
                    "description": "What the related diagnostic says.",
                    "type": "string",
                },
                "code": {
                    "description": "The related diagnostic's own code, which is free text. It is present only when the diagnostic has one.",
                    "type": "string",
                },
                "severity": {
                    "description": "How severe the related diagnostic is. A diagnostic that gives no severity is an error.",
                    "enum": ["error", "warning", "advice"],
                },
                "cause": {
                    "description": "The errors below the related diagnostic, joined with \": \". It is present only when there is one.",
                    "type": "string",
                },
                "labels": labels_schema(),
                "help": {
                    "description": "The related diagnostic's advice. It is present only when it gives some.",
                    "type": "string",
                },
                "url": {
                    "description": "A page that explains the related diagnostic. It is present only when it has one.",
                    "type": "string",
                },
                "related": {
                    "description": "The diagnostics related to this one, in the same shape, to at most eight levels below the failure.",
                    "type": "array",
                    "items": { "$ref": "#related" },
                },
            },
            "required": ["message", "severity"],
        },
    })
}
