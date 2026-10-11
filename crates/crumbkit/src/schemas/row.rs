//! The rows that `schemas list` and `schemas show` print.

use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::Serialize;
use serde::ser::{SerializeStruct, Serializer};
use serde_json::Value;

use super::Served;
use crate::Row;

/// One document that the tool prints, as `schemas list` prints it.
///
/// The text table shows the id and the description, and `--long` adds the
/// command lines that print the document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemaRow {
    // The `JsonSchema` impl below describes each field.
    id: String,
    description: &'static str,
    commands: Vec<String>,
}

impl SchemaRow {
    /// Returns the row of `served`.
    pub(super) fn new(served: &Served) -> Self {
        Self {
            id: served.id(),
            description: served.description(),
            commands: served.commands().to_vec(),
        }
    }
}

impl Row for SchemaRow {
    const COLUMNS: &'static [&'static str] = &["id", "description"];
    const LONG_COLUMNS: &'static [&'static str] = &["commands"];
}

/// Serializes the row as an object with every field.
///
/// The `Serialize` impls of this module are written by hand so crumbkit
/// needs no `serde_derive`.
impl Serialize for SchemaRow {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut row = serializer.serialize_struct("SchemaRow", 3)?;
        row.serialize_field("id", &self.id)?;
        row.serialize_field("description", self.description)?;
        row.serialize_field("commands", &self.commands)?;
        row.end()
    }
}

/// The `JsonSchema` impls of this module are written by hand so crumbkit
/// needs no schemars `derive` feature.
impl JsonSchema for SchemaRow {
    fn schema_name() -> Cow<'static, str> {
        "SchemaRow".into()
    }

    fn schema_id() -> Cow<'static, str> {
        "crumbkit::schemas::SchemaRow".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "description": "One JSON document that the tool prints.",
            "type": "object",
            "properties": {
                "id": {
                    "description": "The document's id, such as `shelf/books-list/v1`, which `schemas show` takes.",
                    "type": "string",
                },
                "description": {
                    "description": "What the document holds, in one line.",
                    "type": "string",
                },
                "commands": {
                    "description": "The command lines that print the document. It is empty for the failure document, which every command prints when it fails with --json.",
                    "type": "array",
                    "items": { "type": "string" },
                },
            },
            "required": ["id", "description", "commands"],
        })
    }
}

/// The JSON Schema of one document, as `schemas show` prints it.
///
/// The text table prints the schema as one line of compact JSON, and
/// `--json` prints it as an object, which `jq '.data[0].schema'` extracts.
#[derive(Clone, Debug, PartialEq)]
pub struct SchemaShowRow {
    // The `JsonSchema` impl below describes each field.
    id: String,
    schema: Value,
}

impl SchemaShowRow {
    /// Returns the row of `served`, whose schema it builds.
    pub(super) fn new(served: &Served) -> Self {
        Self {
            id: served.id(),
            schema: served.schema().to_value(),
        }
    }
}

impl Row for SchemaShowRow {
    const COLUMNS: &'static [&'static str] = &["id", "schema"];
}

impl Serialize for SchemaShowRow {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut row = serializer.serialize_struct("SchemaShowRow", 2)?;
        row.serialize_field("id", &self.id)?;
        row.serialize_field("schema", &self.schema)?;
        row.end()
    }
}

impl JsonSchema for SchemaShowRow {
    fn schema_name() -> Cow<'static, str> {
        "SchemaShowRow".into()
    }

    fn schema_id() -> Cow<'static, str> {
        "crumbkit::schemas::SchemaShowRow".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "description": "The JSON Schema of one document that the tool prints.",
            "type": "object",
            "properties": {
                "id": {
                    "description": "The document's id, such as `shelf/books-list/v1`.",
                    "type": "string",
                },
                "schema": {
                    "description": "The document's 2020-12 JSON Schema.",
                    "type": ["object", "boolean"],
                },
            },
            "required": ["id", "schema"],
        })
    }
}
