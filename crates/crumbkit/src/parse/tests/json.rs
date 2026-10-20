//! Tests for the `--json` of a refused command line: it makes the refusal
//! print as a JSON document only where a command on the typed path declares
//! the flag.

use std::process::ExitCode;

use clap::{Arg, ArgAction, Command};

use super::{admits, parse_tree};

/// A tool without view flags whose leaf `thing` has a `--quiet` flag and no
/// `--json`.
fn plain() -> Command {
    Command::new("tool").subcommand(
        Command::new("thing").arg(Arg::new("quiet").long("quiet").action(ArgAction::SetTrue)),
    )
}

/// Returns the `--json` flag, global when `global` is true.
fn json_flag(global: bool) -> Arg {
    Arg::new("json")
        .long("json")
        .global(global)
        .action(ArgAction::SetTrue)
}

#[test]
fn a_json_word_on_a_leaf_that_declares_no_json_flag_is_refused_as_text_and_asks_for_no_json() {
    let refused = ["tool", "thing", "--json"];
    admits(plain(), &refused, &refused[..2]);
    let parsed = parse_tree(plain(), &refused);

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    assert_eq!(parsed.json, Some(false));
    assert_eq!(parsed.out, "");
    assert!(
        parsed
            .err
            .starts_with("error: unexpected argument '--json' found"),
        "{}",
        parsed.err
    );
}

#[test]
fn a_global_json_flag_on_the_root_makes_a_refusal_on_the_leaf_print_as_json() {
    let tool = || plain().arg(json_flag(true));
    let refused = ["tool", "thing", "--bogus", "--json"];
    admits(tool(), &refused, &["tool", "thing", "--quiet", "--json"]);
    let parsed = parse_tree(tool(), &refused);

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    assert_eq!(parsed.json, Some(true));
    assert!(
        parsed.out.starts_with(r#"{"schema":"tool/failure/v1""#),
        "{}",
        parsed.out
    );
    assert_eq!(parsed.err, "");
}

#[test]
fn a_json_flag_typed_before_the_subcommand_counts_where_the_root_declares_it() {
    let tool = || plain().arg(json_flag(false));
    let refused = ["tool", "--json", "thing", "--bogus"];
    admits(tool(), &refused, &["tool", "--json", "thing", "--quiet"]);
    let parsed = parse_tree(tool(), &refused);

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    assert_eq!(parsed.json, Some(true));
    assert!(
        parsed.out.starts_with(r#"{"schema":"tool/failure/v1""#),
        "{}",
        parsed.out
    );
}
