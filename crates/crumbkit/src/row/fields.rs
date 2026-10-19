//! The fields of a row type, read from its JSON Schema, which `--fields` and
//! a leaf's columns name.

use schemars::Schema;
use serde_json::Value;

/// Returns the properties of the row type whose JSON Schema is `schema`, as
/// pairs of a field name and the schema of the field, each name once in the
/// order it is first found.
///
/// The properties are those at the top level of the schema, followed by
/// those of each variant under `oneOf` and then under `anyOf`, in order. A
/// variant's own properties come before those of the definition that its
/// `$ref` names, which schemars keeps under the schema's `$defs`. A `$ref`
/// that does not point into the schema itself adds nothing.
pub(crate) fn properties(schema: &Schema) -> Vec<(&str, &Value)> {
    let root = schema.as_value();
    let mut objects = vec![root];
    for keyword in ["oneOf", "anyOf"] {
        let variants = root.get(keyword).and_then(Value::as_array);
        for variant in variants.into_iter().flatten() {
            objects.push(variant);
            // A local `$ref` is a JSON Pointer into the schema after its `#`.
            let reference = variant.get("$ref").and_then(Value::as_str);
            let target = reference
                .and_then(|reference| reference.strip_prefix('#'))
                .and_then(|pointer| root.pointer(pointer));
            objects.extend(target);
        }
    }

    let mut found: Vec<(&str, &Value)> = Vec::new();
    for object in objects {
        let Some(properties) = object.get("properties").and_then(Value::as_object) else {
            continue;
        };
        for (name, property) in properties {
            if !found.iter().any(|(seen, _)| seen == name) {
                found.push((name, property));
            }
        }
    }
    found
}

/// Reports whether `property`, the JSON Schema of one field, allows a
/// string, a boolean or null, in its own `type` or in the `type` of a
/// variant under its `anyOf` or `oneOf`.
pub(crate) fn can_be_empty(property: &Value) -> bool {
    let mut schemas = vec![property];
    for keyword in ["anyOf", "oneOf"] {
        if let Some(variants) = property.get(keyword).and_then(Value::as_array) {
            schemas.extend(variants);
        }
    }
    let empty = |kind: &Value| matches!(kind.as_str(), Some("string" | "boolean" | "null"));
    schemas.iter().any(|schema| match schema.get("type") {
        Some(Value::Array(kinds)) => kinds.iter().any(empty),
        Some(kind) => empty(kind),
        None => false,
    })
}
