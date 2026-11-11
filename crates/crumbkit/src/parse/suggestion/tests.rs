//! Tests for the names that a refusal suggests, through `parse_to`.

use crate::parse::tests::parse_tree_to;
use clap::error::ErrorKind;
use clap::{Arg, ArgAction, Command};

use crate::{Sink, Tool};

/// Returns what `parse_to` prints on standard error for `args`, which
/// `command` refuses.
fn refusal(command: Command, args: &[&str]) -> String {
    let mut err = Vec::new();
    let _ = parse_tree_to::<()>(
        &mut Sink::new(Vec::new(), &mut err),
        command,
        &Tool::new(),
        args,
    );
    String::from_utf8(err).unwrap()
}

/// A tool whose `books` command has a `--limit` option with the hidden
/// alias `--max` and a hidden `--legacy` flag, and whose hidden `purge`
/// command has the alias `wipe` and a `--max` option.
fn tree() -> Command {
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
    let err = refusal(tree(), &["shelf", "--maxx", "5", "books"]);

    assert!(
        err.starts_with("error: unexpected argument '--maxx' found; 'books --limit' exists\n"),
        "{err}"
    );
}

#[test]
fn a_tip_that_names_a_flag_of_a_hidden_subcommand_is_left_out() {
    let err = refusal(tree(), &["shelf", "--maxx", "5", "purge"]);

    assert!(
        err.starts_with("error: unexpected argument '--maxx' found\n"),
        "{err}"
    );
}

#[test]
fn a_tip_that_names_a_hidden_flag_of_a_subcommand_is_left_out() {
    let err = refusal(tree(), &["shelf", "--legac", "books"]);

    assert!(
        err.starts_with("error: unexpected argument '--legac' found\n"),
        "{err}"
    );
}

#[test]
fn a_refusal_drops_a_suggested_alias_whose_command_is_hidden() {
    let err = refusal(tree(), &["shelf", "wip"]);

    assert!(
        err.starts_with("error: unrecognized subcommand 'wip'\n"),
        "{err}"
    );
}

#[test]
fn a_refusal_drops_a_suggested_flag_that_is_hidden() {
    let err = refusal(tree(), &["shelf", "books", "--legac"]);

    assert!(
        err.starts_with("error: unexpected argument '--legac' found\n"),
        "{err}"
    );
}

/// Returns the id of the flag that `nearest_flag` finds when `tool` refuses
/// `args` at `books`.
fn nearest(args: &[&str]) -> Option<String> {
    let mut command = tree();
    let error = command.try_get_matches_from_mut(args).unwrap_err();
    super::nearest_flag(&command, &["books".to_owned()], &error).map(|arg| arg.get_id().to_string())
}

#[test]
fn the_nearest_flag_of_a_suggested_hidden_alias_is_the_flag_that_the_alias_belongs_to() {
    assert_eq!(
        nearest(&["shelf", "books", "--maxx", "5"]).as_deref(),
        Some("limit")
    );
}

#[test]
fn a_suggested_hidden_flag_gives_no_nearest_flag() {
    assert_eq!(nearest(&["shelf", "books", "--legac"]), None);
}

#[test]
fn a_message_that_only_reads_like_claps_tip_names_no_nearest_flag() {
    let command = tree();
    let error = clap::Error::raw(
        ErrorKind::UnknownArgument,
        "unexpected argument '--limi' found\n\n  tip: a similar argument exists: '--limit'\n",
    );

    let nearest = super::nearest_flag(&command, &["books".to_owned()], &error);

    assert_eq!(nearest.map(Arg::get_id), None);
}

/// A tool whose `books` command has the alias `boots`, beside a `boats`
/// command, and whose `lend` command takes a `--limit` option with the
/// hidden alias `--max`, a `--format` option with possible values and the
/// words to pass on.
fn lending() -> Command {
    Command::new("shelf")
        .subcommand(Command::new("books").alias("boots"))
        .subcommand(Command::new("boats"))
        .subcommand(
            Command::new("lend")
                .arg(Arg::new("limit").long("limit").alias("max"))
                .arg(
                    Arg::new("format")
                        .long("format")
                        .value_parser(["json", "text"]),
                )
                .arg(Arg::new("words").num_args(0..).last(true)),
        )
}

#[test]
fn several_suggested_subcommands_name_each_real_command_once() {
    let err = refusal(lending(), &["shelf", "boos"]);

    assert!(
        err.starts_with(
            "error: unrecognized subcommand 'boos'; some similar subcommands exist: 'boats', 'books'\n"
        ),
        "{err}"
    );
}

#[test]
fn a_suggested_hidden_alias_names_the_real_flag_before_the_tip_to_pass_a_value() {
    let err = refusal(lending(), &["shelf", "lend", "--maxx"]);

    assert!(
        err.starts_with(
            "error: unexpected argument '--maxx' found; a similar argument exists: '--limit'; \
             to pass '--maxx' as a value, use '-- --maxx'\n"
        ),
        "{err}"
    );
}

#[test]
fn a_tip_that_suggests_a_similar_value_stays_as_clap_wrote_it() {
    let err = refusal(lending(), &["shelf", "lend", "--format", "jsn"]);

    assert!(err.contains("; a similar value exists: 'json'\n"), "{err}");
}
