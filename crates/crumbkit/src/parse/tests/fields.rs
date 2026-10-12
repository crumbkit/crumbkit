//! Tests for the fields of a refused command line, which name what clap
//! refused.

use clap::{Arg, value_parser};

use super::{parse, parse_tree, tool};

/// Returns the `fields` object of the failure document in `out`, or `None`
/// when the document has none.
fn fields(out: &str) -> Option<serde_json::Value> {
    let document: serde_json::Value = serde_json::from_str(out).unwrap();
    document.get("fields").cloned()
}

#[test]
fn a_refused_value_names_the_flag_without_its_value_name_and_the_value() {
    let limit = Arg::new("limit")
        .long("limit")
        .value_name("N")
        .global(true)
        .value_parser(value_parser!(u32));

    let parsed = parse_tree(
        tool().arg(limit),
        &[
            "tool", "planets", "show", "Mars", "--limit", "many", "--json",
        ],
    );

    assert_eq!(
        fields(&parsed.out),
        Some(serde_json::json!({"argument": "--limit", "value": "many"}))
    );
}

#[test]
fn an_unknown_subcommand_names_the_subcommand() {
    let parsed = parse(&["tool", "planets", "shw", "--json"]);

    assert_eq!(
        fields(&parsed.out),
        Some(serde_json::json!({"subcommand": "shw"}))
    );
}

#[test]
fn a_missing_required_argument_names_the_argument() {
    let parsed = parse(&["tool", "planets", "show", "--json"]);

    assert_eq!(
        fields(&parsed.out),
        Some(serde_json::json!({"argument": "<NAME>..."}))
    );
}

#[test]
fn a_command_that_needs_a_subcommand_has_no_fields() {
    let parsed = parse(&["tool", "planets", "--json"]);

    assert_eq!(fields(&parsed.out), None);
}
