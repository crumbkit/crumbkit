//! Tests for the arguments of `commands list` as JSON and their schema.

use serde_json::{Value, json};

use super::{args, listed, row};
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

#[test]
fn an_argument_holds_what_its_help_shows() {
    let rows = listed(&["books"]);

    assert_eq!(
        row(&rows, "shelf books list")["arguments"],
        json!([
            {
                "name": "--limit",
                "short": "-n",
                "value_names": ["N"],
                "required": false,
                "many": false,
                "defaults": ["4"],
                "possible_values": [],
                "value_hint": null,
                "help": "Print at most N books",
                "long_help": "Print at most N books.\n\nThe default is 4.",
                "global": false,
                "one_of": [],
            },
            {
                "name": "--form",
                "short": null,
                "value_names": ["form"],
                "required": false,
                "many": true,
                "defaults": [],
                "possible_values": ["novel", "short stories"],
                "value_hint": null,
                "help": null,
                "long_help": null,
                "global": false,
                "one_of": [],
            },
        ])
    );
    assert_eq!(
        row(&rows, "shelf books show")["arguments"],
        json!([
            {
                "name": "<ID>",
                "short": null,
                "value_names": ["ID"],
                "required": false,
                "many": true,
                "defaults": [],
                "possible_values": [],
                "value_hint": null,
                "help": null,
                "long_help": null,
                "global": false,
                "one_of": ["--from"],
            },
            {
                "name": "--from",
                "short": null,
                "value_names": ["FILE"],
                "required": false,
                "many": false,
                "defaults": [],
                "possible_values": [],
                "value_hint": null,
                "help": null,
                "long_help": null,
                "global": false,
                "one_of": ["<ID>"],
            },
        ])
    );
}
