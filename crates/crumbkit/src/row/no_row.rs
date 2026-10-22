//! The row type of a result without rows.

use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Serialize, Serializer};

use super::Row;

/// The row type of a result without rows, which is a
/// [`Prose`](crate::Prose). It has no values and no columns, so a result of
/// this row type never holds a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoRow {}

impl Row for NoRow {
    const COLUMNS: &'static [&'static str] = &[];
}

impl Serialize for NoRow {
    fn serialize<S: Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        match *self {}
    }
}

/// Describes `NoRow` as the schema `false`, which no value matches, because
/// a result of this row type never holds a row. The schema is inline, so a
/// generator returns `false` itself and adds no definition.
impl JsonSchema for NoRow {
    fn inline_schema() -> bool {
        true
    }

    fn schema_name() -> Cow<'static, str> {
        "NoRow".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        Schema::from(false)
    }
}
