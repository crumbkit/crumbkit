//! The `required` list of a row in a document whose JSON prints only the
//! row's columns without `--full` (ADR-0110).
//!
//! RUST-10: the cut changes `required` lists only. A column that the row
//! type skips when it is empty prints `null` in the compact document, and
//! the property's type admits that only when it accepts null. A compact
//! row of an untagged enum can match more than one `oneOf` variant after
//! the cut and then fails `oneOf`. No row type in this repository has
//! either, so the cut leaves both alone; adding `null` to such a
//! property's type, or turning `oneOf` into `anyOf` for an untagged enum,
//! replaces it when a row type needs it.

use std::collections::HashMap;

use schemars::Schema;
use serde_json::{Map, Value};

/// Cuts the `required` list of the row in the document schema `schema` to
/// the names in `columns`, so a document that holds only the columns
/// matches the schema.
///
/// The row is the schema of `data`, or of its `items` when `items` is set.
/// A row whose schema names a definition under `$defs` is cut in that
/// definition when nothing else names it, and otherwise in a copy of the
/// definition that replaces the reference. An enum row has its `oneOf` or
/// `anyOf` variants cut one by one. A name in `columns` that the row does
/// not require stays optional, because `--json --full` leaves out a field
/// that the row type skips.
pub(super) fn require_columns(schema: &mut Schema, items: bool, columns: &[&str]) {
    let mut references = HashMap::new();
    count_references(schema.as_value(), &mut references);
    let Some(object) = schema.as_object_mut() else {
        return;
    };
    let mut definitions = match object.remove("$defs") {
        Some(Value::Object(definitions)) => definitions,
        _ => Map::new(),
    };
    let data = object
        .get_mut("properties")
        .and_then(|properties| properties.get_mut("data"));
    let row = if items {
        data.and_then(|data| data.get_mut("items"))
    } else {
        data
    };
    if let Some(row) = row {
        cut(row, &mut definitions, &references, columns);
    }
    if !definitions.is_empty() {
        object.insert("$defs".to_owned(), Value::Object(definitions));
    }
}

/// Cuts the `required` list of the row schema `row`, and of each of its
/// variants, to `columns`, following a reference into `definitions` as
/// [`require_columns`] describes.
fn cut(
    row: &mut Value,
    definitions: &mut Map<String, Value>,
    references: &HashMap<String, usize>,
    columns: &[&str],
) {
    if let Some(name) = definition_name(row) {
        let Some(mut definition) = definitions.get(&name).cloned() else {
            return;
        };
        cut(&mut definition, definitions, references, columns);
        if references.get(&name) == Some(&1) {
            definitions.insert(name, definition);
        } else if let (Some(copy), Some(keywords)) = (definition.as_object_mut(), row.as_object()) {
            for (keyword, value) in keywords {
                if keyword != "$ref" {
                    copy.insert(keyword.clone(), value.clone());
                }
            }
            *row = definition;
        }
        return;
    }
    let Some(object) = row.as_object_mut() else {
        return;
    };
    if let Some(Value::Array(required)) = object.get_mut("required") {
        required.retain(|name| name.as_str().is_some_and(|name| columns.contains(&name)));
    }
    for keyword in ["oneOf", "anyOf"] {
        if let Some(Value::Array(variants)) = object.get_mut(keyword) {
            for variant in variants {
                cut(variant, definitions, references, columns);
            }
        }
    }
}

/// Returns the name of the definition under `$defs` that `schema` names
/// with `$ref`, or `None` when it names none.
fn definition_name(schema: &Value) -> Option<String> {
    let reference = schema.get("$ref")?.as_str()?;
    reference.strip_prefix("#/$defs/").map(str::to_owned)
}

/// Counts in `references` each definition under `$defs` that `value` or
/// its descendants name with `$ref`.
fn count_references(value: &Value, references: &mut HashMap<String, usize>) {
    match value {
        Value::Object(object) => {
            if let Some(name) = definition_name(value) {
                *references.entry(name).or_default() += 1;
            }
            for child in object.values() {
                count_references(child, references);
            }
        }
        Value::Array(array) => {
            for child in array {
                count_references(child, references);
            }
        }
        _ => {}
    }
}
