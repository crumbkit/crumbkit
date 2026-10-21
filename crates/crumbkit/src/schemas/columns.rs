//! The `required` list of a row in a document whose JSON prints only the
//! row's columns without `--full` (ADR-0110), and the null that a column
//! the row type skips prints there (ADR-0113), or that a column or the
//! children field prints for a variant without the field (ADR-0132).
//!
//! RUST-10: a compact row of an untagged enum can match more than one
//! `oneOf` variant after the cut and then fails `oneOf`. No row type in
//! this repository is an untagged enum, so the cut leaves `oneOf` alone;
//! turning `oneOf` into `anyOf` for an untagged enum replaces it when a row
//! type needs it.

use std::collections::HashMap;

use schemars::Schema;
use serde_json::{Map, Value, json};

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
/// that the row type skips, and its property admits null as [`cut`]
/// describes. `columns` holds `Row::CHILDREN` beside the columns, and a
/// name in `columns` that a row or variant has no property for gains a
/// property that admits only null, as [`cut`] describes.
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
        cut(row, &mut definitions, &references, columns, &[]);
    }
    if !definitions.is_empty() {
        object.insert("$defs".to_owned(), Value::Object(definitions));
    }
}

/// Cuts the `required` list of the row schema `row`, and of each of its
/// variants, to `columns`, following a reference into `definitions` as
/// [`require_columns`] describes.
///
/// A name in `columns` that the row's `required` list lacks before the cut
/// and that the row's `properties` holds names a field that the row type
/// skips when it is empty. The compact document prints `null` for that
/// field, so its property admits null as [`admit_null`] describes. A
/// property for a column that the row requires keeps its schema. A name in
/// `columns` that the row's `properties` lacks names a field that the row,
/// or one of its variants, does not have, and the compact document prints
/// `null` for it too, so the row gains a property that admits only null as
/// [`add_null_properties`] describes. `around` holds the property names
/// that the schemas around `row` declare beside it, such as the fields of a
/// struct whose flattened enum gives `row` as one variant.
fn cut(
    row: &mut Value,
    definitions: &mut Map<String, Value>,
    references: &HashMap<String, usize>,
    columns: &[&str],
    around: &[String],
) {
    if let Some(name) = definition_name(row) {
        let Some(mut definition) = definitions.get(&name).cloned() else {
            return;
        };
        cut(&mut definition, definitions, references, columns, around);
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
    let required = match object.get_mut("required") {
        Some(Value::Array(required)) => {
            let before = required.clone();
            required.retain(|name| name.as_str().is_some_and(|name| columns.contains(&name)));
            before
        }
        _ => Vec::new(),
    };
    if let Some(Value::Object(properties)) = object.get_mut("properties") {
        for &column in columns {
            if !required.iter().any(|name| name == column)
                && let Some(property) = properties.get_mut(column)
            {
                admit_null(property);
            }
        }
    }
    let mut inside = around.to_vec();
    if let Some(Value::Object(properties)) = object.get("properties") {
        inside.extend(properties.keys().cloned());
    }
    declared(
        object,
        definitions,
        &["allOf"],
        &mut Vec::new(),
        &mut inside,
    );
    let mut present = inside.clone();
    declared(
        object,
        definitions,
        &["oneOf", "anyOf"],
        &mut Vec::new(),
        &mut present,
    );
    add_null_properties(object, columns, &present);
    for keyword in ["oneOf", "anyOf"] {
        if let Some(Value::Array(variants)) = object.get_mut(keyword) {
            for variant in variants {
                cut(variant, definitions, references, columns, &inside);
            }
        }
    }
}

/// Gives the object schema `object` the property `{"type": "null"}` for
/// each name in `columns` that neither its `properties` nor `present`
/// holds, so a row whose type has no such field matches the null that the
/// compact document prints.
///
/// Only a schema with a `properties` map, or with `additionalProperties:
/// false`, gains the property. A schema with neither, such as an enum's
/// own schema whose variants sit under `oneOf`, already admits a field
/// that it does not describe, and each of its variants gains the property
/// when [`cut`] reaches it. `present` holds the names that a schema around
/// `object` or one of its `oneOf`, `anyOf` or `allOf` members declares,
/// because a null property beside that schema would refuse its value.
fn add_null_properties(object: &mut Map<String, Value>, columns: &[&str], present: &[String]) {
    let closed = object.get("additionalProperties") == Some(&Value::Bool(false));
    if !closed && !matches!(object.get("properties"), Some(Value::Object(_))) {
        return;
    }
    let Value::Object(properties) = object
        .entry("properties")
        .or_insert_with(|| Value::Object(Map::new()))
    else {
        return;
    };
    for &column in columns {
        if !properties.contains_key(column) && !present.iter().any(|name| name == column) {
            properties.insert(column.to_owned(), json!({"type": "null"}));
        }
    }
}

/// Adds to `names` each property name that the members of the schema
/// `object` under `keywords` declare, and the names that their own members
/// under `keywords` declare, following `$ref` into `definitions`. `visited`
/// holds the definitions already followed, so a definition that names
/// itself is read once.
fn declared(
    object: &Map<String, Value>,
    definitions: &Map<String, Value>,
    keywords: &[&str],
    visited: &mut Vec<String>,
    names: &mut Vec<String>,
) {
    for keyword in keywords {
        let Some(Value::Array(members)) = object.get(*keyword) else {
            continue;
        };
        for member in members {
            let member = match definition_name(member) {
                Some(name) if visited.contains(&name) => continue,
                Some(name) => {
                    let definition = definitions.get(&name);
                    visited.push(name);
                    definition
                }
                None => Some(member),
            };
            let Some(member) = member.and_then(Value::as_object) else {
                continue;
            };
            if let Some(Value::Object(properties)) = member.get("properties") {
                names.extend(properties.keys().cloned());
            }
            declared(member, definitions, keywords, visited, names);
        }
    }
}

/// Lets the property schema `property` admit null, in the shapes that
/// schemars gives `Option<T>`.
///
/// A property whose `type` already names `"null"`, or that has an `anyOf`
/// member whose `type` names `"null"`, is unchanged. A property with a
/// `type` and neither `enum` nor `const` gains `"null"` in its `type`, so
/// `"string"` becomes `["string", "null"]`. Any other property becomes an
/// `anyOf` of itself without its `description` and `{"type": "null"}`, and
/// keeps its `description` beside the `anyOf`. A property that is not an
/// object, such as `true`, is unchanged.
fn admit_null(property: &mut Value) {
    let Some(object) = property.as_object_mut() else {
        return;
    };
    let names_null = |schema: &Map<String, Value>| match schema.get("type") {
        Some(Value::String(name)) => name == "null",
        Some(Value::Array(names)) => names.iter().any(|name| name == "null"),
        _ => false,
    };
    let member_admits_null = match object.get("anyOf") {
        Some(Value::Array(members)) => members.iter().filter_map(Value::as_object).any(names_null),
        _ => false,
    };
    if names_null(object) || member_admits_null {
        return;
    }
    let plain = !object.contains_key("enum") && !object.contains_key("const");
    match object.get_mut("type") {
        Some(Value::String(name)) if plain => {
            let name = std::mem::take(name);
            object.insert("type".to_owned(), json!([name, "null"]));
        }
        Some(Value::Array(names)) if plain => names.push(json!("null")),
        _ => {
            let description = object.remove("description");
            let mut wrapped = Map::new();
            wrapped.insert(
                "anyOf".to_owned(),
                json!([Value::Object(std::mem::take(object)), {"type": "null"}]),
            );
            if let Some(description) = description {
                wrapped.insert("description".to_owned(), description);
            }
            *object = wrapped;
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
