//! Tests for the refusal of a `--limit` above the bound of a command's
//! `Page`, whose recovery runs the caller's words with `--all`.

use std::process::ExitCode;

use clap::{Args as _, Command};
use serde_json::{Value, json};

use super::{admits, parse_tree, tool};
use crate::Page;

/// Returns `tool()` with the leaf `planets list`, whose page takes at most
/// 10 rows.
fn bounded() -> Command {
    tool().mut_subcommand("planets", |planets| {
        planets.subcommand(Page::<4, 10>::augment_args(
            Command::new("list").about("List the planets"),
        ))
    })
}

#[test]
fn a_limit_above_the_bound_is_refused_with_a_recovery_that_runs_all() {
    let refused = ["tool", "planets", "list", "--limit", "11"];

    let parsed = parse_tree(bounded(), &refused);

    let expected = "\
error: invalid value '11' for '--limit <N>': --limit takes a whole number from 1 to 10
  help:  run `tool planets list --all`
  help:  run `tool planets list --limit 10`
  code:  tool::cli::usage
";
    assert_eq!(parsed.err, expected);
    assert_eq!(parsed.out, "");
    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    admits(
        bounded(),
        &refused,
        &["tool", "planets", "list", "--limit", "10"],
    );
}

#[test]
fn the_refusal_document_names_the_bound_and_keeps_the_callers_other_words() {
    let parsed = parse_tree(
        bounded(),
        &["tool", "planets", "list", "--limit", "11", "--json"],
    );

    let document: Value = serde_json::from_str(&parsed.out).unwrap();
    assert_eq!(
        document["fields"],
        json!({"argument": "--limit", "value": "11", "most": 10})
    );
    assert_eq!(
        document["recovery"],
        json!({"kind": "run", "call": "tool planets list --all --json"})
    );
    assert_eq!(
        document["alternatives"],
        json!([{"kind": "run", "call": "tool planets list --limit 10 --json"}])
    );
}

#[test]
fn a_limit_written_with_an_equals_sign_is_replaced_by_all() {
    let parsed = parse_tree(bounded(), &["tool", "planets", "list", "--limit=11"]);

    assert!(
        parsed
            .err
            .contains("  help:  run `tool planets list --all`\n"),
        "{}",
        parsed.err
    );
}

#[test]
fn a_limit_of_0_on_a_bounded_page_states_the_range_and_keeps_the_help_recovery() {
    let parsed = parse_tree(bounded(), &["tool", "planets", "list", "--limit", "0"]);

    let expected = "\
error: invalid value '0' for '--limit <N>': --limit takes a whole number from 1 to 10
  help:  run `tool planets list -h`
  code:  tool::cli::usage
";
    assert_eq!(parsed.err, expected);
}

#[test]
fn a_limit_that_is_not_a_number_on_a_bounded_page_states_the_range_and_keeps_the_help_recovery() {
    let parsed = parse_tree(bounded(), &["tool", "planets", "list", "--limit", "abc"]);

    let expected = "\
error: invalid value 'abc' for '--limit <N>': --limit takes a whole number from 1 to 10
  help:  run `tool planets list -h`
  code:  tool::cli::usage
";
    assert_eq!(parsed.err, expected);
}

#[test]
fn a_negative_limit_on_a_bounded_page_states_the_range_and_keeps_the_help_recovery() {
    let parsed = parse_tree(bounded(), &["tool", "planets", "list", "--limit=-1"]);

    let expected = "\
error: invalid value '-1' for '--limit <N>': --limit takes a whole number from 1 to 10
  help:  run `tool planets list -h`
  code:  tool::cli::usage
";
    assert_eq!(parsed.err, expected);
}
