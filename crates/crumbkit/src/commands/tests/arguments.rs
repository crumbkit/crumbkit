//! Tests for the arguments of `commands list` as JSON and their schema.

use serde_json::{Value, json};

use super::args;
use crate::commands::CommandsList;

#[test]
fn the_arguments_serialize_as_the_path_beside_the_page_flags() {
    let value = serde_json::to_value(args(&["books", "list", "--all"])).unwrap();

    assert_eq!(
        value,
        json!({ "path": ["books", "list"], "limit": 25, "all": true })
    );
}

#[test]
fn the_arguments_schema_describes_the_path_and_the_page_flags_and_requires_each() {
    let schema = schemars::schema_for!(CommandsList);

    let mut properties: Vec<&String> = schema
        .get("properties")
        .and_then(Value::as_object)
        .unwrap()
        .keys()
        .collect();
    properties.sort();
    assert_eq!(properties, ["all", "limit", "path"]);
    let mut required: Vec<&str> = schema
        .get("required")
        .and_then(Value::as_array)
        .unwrap()
        .iter()
        .map(|name| name.as_str().unwrap())
        .collect();
    required.sort_unstable();
    assert_eq!(required, ["all", "limit", "path"]);
}
