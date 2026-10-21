//! The check that crumbkit's own tests use to tell whether a JSON value
//! matches a JSON Schema that crumbkit builds.

#[cfg(test)]
mod tests;

use serde_json::{Map, Value};

/// Returns `Ok` when `value` matches `schema` under JSON Schema draft
/// 2020-12, and otherwise an error that names the path to the first value
/// that fails and the keyword it fails.
///
/// `$ref` names a definition under the `$defs` of `schema`.
//
// RUST-10: this check handles only the keywords that the schemas crumbkit
// builds for its documents use: `$ref` to `#/$defs/`, `type`, `const`,
// `enum`, `properties`, `required`, `additionalProperties`, `items`,
// `oneOf`, `anyOf`, `allOf` and `minimum`, and it skips the annotations
// `title`, `description`, `format`, `$schema` and `$defs`. It returns an
// error on any other keyword, so a schema that needs more fails the test
// and never passes unchecked. A maintained validator crate, such as
// `jsonschema`, replaces it when a test needs another keyword.
pub(crate) fn conforms(schema: &Value, value: &Value) -> Result<(), String> {
    check(schema, schema, value, "$")
}

/// Checks `value` at `path` against `schema`, resolving each `$ref` in
/// the `$defs` of `root`.
fn check(root: &Value, schema: &Value, value: &Value, path: &str) -> Result<(), String> {
    let object = match schema {
        Value::Bool(true) => return Ok(()),
        Value::Bool(false) => return Err(format!("{path}: the schema `false` admits nothing")),
        Value::Object(object) => object,
        _ => return Err(format!("{path}: the schema is not an object")),
    };
    for (keyword, rule) in object {
        match keyword.as_str() {
            "$ref" => check(root, resolve(root, rule, path)?, value, path)?,
            "type" => check_type(rule, value, path)?,
            "const" if value != rule => {
                return Err(format!("{path}: {value} is not the const {rule}"));
            }
            "enum" if !rule.as_array().is_some_and(|names| names.contains(value)) => {
                return Err(format!("{path}: {value} is not in the enum {rule}"));
            }
            "const"
            | "enum"
            | "title"
            | "description"
            | "format"
            | "$schema"
            | "$defs"
            | "required"
            | "additionalProperties" => {}
            "minimum" => {
                if let (Some(number), Some(minimum)) = (value.as_f64(), rule.as_f64())
                    && number < minimum
                {
                    return Err(format!("{path}: {value} is below the minimum {rule}"));
                }
            }
            "properties" => check_properties(root, object, value, path)?,
            "items" => {
                if let Value::Array(items) = value {
                    for (index, item) in items.iter().enumerate() {
                        check(root, rule, item, &format!("{path}[{index}]"))?;
                    }
                }
            }
            "oneOf" | "anyOf" | "allOf" => check_members(root, keyword, rule, value, path)?,
            other => return Err(format!("{path}: the keyword `{other}` is not handled")),
        }
    }
    if !object.contains_key("properties") {
        check_properties(root, object, value, path)?;
    }
    Ok(())
}

/// Returns the definition that the `$ref` value `rule` names.
fn resolve<'a>(root: &'a Value, rule: &Value, path: &str) -> Result<&'a Value, String> {
    rule.as_str()
        .and_then(|reference| reference.strip_prefix("#/$defs/"))
        .and_then(|name| root["$defs"].get(name))
        .ok_or_else(|| format!("{path}: the reference {rule} names no definition"))
}

/// Checks that `value` has a type that the `type` value `rule` names.
fn check_type(rule: &Value, value: &Value, path: &str) -> Result<(), String> {
    let is = |name: &Value| match name.as_str() {
        Some("null") => value.is_null(),
        Some("boolean") => value.is_boolean(),
        Some("object") => value.is_object(),
        Some("array") => value.is_array(),
        Some("string") => value.is_string(),
        Some("number") => value.is_number(),
        Some("integer") => value.is_i64() || value.is_u64(),
        _ => false,
    };
    let names = match rule {
        Value::Array(names) => names.clone(),
        name => vec![name.clone()],
    };
    if names.iter().any(is) {
        Ok(())
    } else {
        Err(format!("{path}: {value} is not of the type {rule}"))
    }
}

/// Checks the `properties`, `required` and `additionalProperties` of the
/// schema `object` against `value`, when `value` is an object.
fn check_properties(
    root: &Value,
    object: &Map<String, Value>,
    value: &Value,
    path: &str,
) -> Result<(), String> {
    let Value::Object(fields) = value else {
        return Ok(());
    };
    let empty = Map::new();
    let properties = object
        .get("properties")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    for name in object
        .get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let name = name.as_str().unwrap_or_default();
        if !fields.contains_key(name) {
            return Err(format!("{path}: the required field `{name}` is missing"));
        }
    }
    for (name, field) in fields {
        let field_path = format!("{path}.{name}");
        match (properties.get(name), object.get("additionalProperties")) {
            (Some(property), _) => check(root, property, field, &field_path)?,
            (None, Some(additional)) => check(root, additional, field, &field_path)?,
            (None, None) => {}
        }
    }
    Ok(())
}

/// Checks `value` against the members of the `oneOf`, `anyOf` or `allOf`
/// value `rule`, as `keyword` asks.
fn check_members(
    root: &Value,
    keyword: &str,
    rule: &Value,
    value: &Value,
    path: &str,
) -> Result<(), String> {
    let members = rule.as_array().map(Vec::as_slice).unwrap_or_default();
    let results: Vec<Result<(), String>> = members
        .iter()
        .map(|member| check(root, member, value, path))
        .collect();
    let matched = results.iter().filter(|result| result.is_ok()).count();
    let failures: Vec<&str> = results
        .iter()
        .filter_map(|result| result.as_ref().err().map(String::as_str))
        .collect();
    let fits = match keyword {
        "oneOf" => matched == 1,
        "anyOf" => matched >= 1,
        _ => failures.is_empty(),
    };
    if fits {
        Ok(())
    } else {
        Err(format!(
            "{path}: {matched} of {} members of `{keyword}` match: {}",
            members.len(),
            failures.join("; ")
        ))
    }
}
