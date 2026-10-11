//! Tests for the names that a refusal suggests, through `parse_to`.

use clap::{Arg, ArgAction, Command};

use crate::help::Guide;
use crate::{Sink, Spelling, parse_to};

/// Returns what `parse_to` prints on standard error for `args`, which
/// `command` refuses.
fn refusal(command: Command, args: &[&str]) -> String {
    let mut err = Vec::new();
    let _ = parse_to::<(), _, _, _, _>(
        &mut Sink::new(Vec::new(), &mut err),
        command,
        &Guide::new(),
        Spelling::COLONS,
        args,
    );
    String::from_utf8(err).unwrap()
}

/// A tool whose `books` command has a `--limit` option with the hidden
/// alias `--max` and a hidden `--legacy` flag, and whose hidden `purge`
/// command has the alias `wipe` and a `--max` option.
fn tool() -> Command {
    Command::new("shelf")
        .subcommand(
            Command::new("books")
                .arg(Arg::new("limit").long("limit").alias("max"))
                .arg(
                    Arg::new("legacy")
                        .long("legacy")
                        .hide(true)
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("purge")
                .hide(true)
                .alias("wipe")
                .arg(Arg::new("max").long("max")),
        )
}

#[test]
fn a_tip_that_names_a_hidden_alias_of_a_subcommand_s_flag_names_the_real_flag() {
    let err = refusal(tool(), &["shelf", "--maxx", "5", "books"]);

    assert!(
        err.starts_with("error: unexpected argument '--maxx' found; 'books --limit' exists\n"),
        "{err}"
    );
}

#[test]
fn a_tip_that_names_a_flag_of_a_hidden_subcommand_is_left_out() {
    let err = refusal(tool(), &["shelf", "--maxx", "5", "purge"]);

    assert!(
        err.starts_with("error: unexpected argument '--maxx' found\n"),
        "{err}"
    );
}

#[test]
fn a_tip_that_names_a_hidden_flag_of_a_subcommand_is_left_out() {
    let err = refusal(tool(), &["shelf", "--legac", "books"]);

    assert!(
        err.starts_with("error: unexpected argument '--legac' found\n"),
        "{err}"
    );
}

#[test]
fn a_refusal_drops_a_suggested_alias_whose_command_is_hidden() {
    let err = refusal(tool(), &["shelf", "wip"]);

    assert!(
        err.starts_with("error: unrecognized subcommand 'wip'\n"),
        "{err}"
    );
}

#[test]
fn a_refusal_drops_a_suggested_flag_that_is_hidden() {
    let err = refusal(tool(), &["shelf", "books", "--legac"]);

    assert!(
        err.starts_with("error: unexpected argument '--legac' found\n"),
        "{err}"
    );
}
