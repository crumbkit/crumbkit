//! Tests for the names check.

use clap::{Arg, ArgAction, Command};
use crumbkit::help::Guide;

use super::check_names;
use crate::allow::{Allow, Rule};
use crate::testing::panic_message;

#[test]
fn commands_and_arguments_with_one_name_and_one_short_form_pass() {
    let shelf = Command::new("shelf").subcommand(
        Command::new("books").subcommand(
            Command::new("list").arg(
                Arg::new("limit")
                    .long("limit")
                    .short('l')
                    .action(ArgAction::Set),
            ),
        ),
    );

    check_names(&shelf, &Guide::new(), &[]);
}

#[test]
fn hidden_aliases_of_a_command_and_of_an_argument_pass() {
    let shelf = Command::new("shelf").subcommand(
        Command::new("books")
            .alias("book")
            .short_flag_alias('k')
            .long_flag_alias("volumes")
            .subcommand(
                Command::new("list").arg(
                    Arg::new("limit")
                        .long("limit")
                        .alias("max")
                        .short_alias('n')
                        .action(ArgAction::Set),
                ),
            ),
    );

    check_names(&shelf, &Guide::new(), &[]);
}

#[test]
fn a_command_with_a_visible_alias_fails() {
    let shelf = Command::new("shelf").subcommand(
        Command::new("books")
            .visible_aliases(["book", "b"])
            .subcommand(Command::new("list")),
    );

    assert_eq!(
        panic_message(|| check_names(&shelf, &Guide::new(), &[])).unwrap(),
        "the command tree breaks crumbkit's naming rules:\n\
         shelf books: the command has the visible alias `book`, and help shows one name for each command\n\
         shelf books: the command has the visible alias `b`, and help shows one name for each command"
    );
}

#[test]
fn a_command_with_a_short_or_a_long_flag_name_fails() {
    let shelf = Command::new("shelf").subcommand(
        Command::new("books")
            .short_flag('B')
            .long_flag("books")
            .subcommand(Command::new("list")),
    );

    assert_eq!(
        panic_message(|| check_names(&shelf, &Guide::new(), &[])).unwrap(),
        "the command tree breaks crumbkit's naming rules:\n\
         shelf books: the command has the short flag name `-B`, and help shows one name for each command\n\
         shelf books: the command has the long flag name `--books`, and help shows one name for each command"
    );
}

#[test]
fn a_command_with_a_visible_short_or_long_flag_alias_fails() {
    let shelf = Command::new("shelf").subcommand(
        Command::new("books")
            .visible_short_flag_alias('k')
            .visible_long_flag_alias("book"),
    );

    assert_eq!(
        panic_message(|| check_names(&shelf, &Guide::new(), &[])).unwrap(),
        "the command tree breaks crumbkit's naming rules:\n\
         shelf books: the command has the visible short flag alias `-k`, and help shows one name for each command\n\
         shelf books: the command has the visible long flag alias `--book`, and help shows one name for each command"
    );
}

#[test]
fn an_argument_with_a_visible_long_or_short_alias_fails() {
    let list = Command::new("list").arg(
        Arg::new("limit")
            .long("limit")
            .short('l')
            .visible_alias("max")
            .visible_short_alias('n')
            .action(ArgAction::Set),
    );

    assert_eq!(
        panic_message(|| check_names(&list, &Guide::new(), &[])).unwrap(),
        "the command tree breaks crumbkit's naming rules:\n\
         list: the argument `--limit` has the visible alias `--max`, and help shows one name for each argument\n\
         list: the argument `--limit` has the visible short alias `-n`, and help shows one name for each argument"
    );
}

#[test]
fn a_global_argument_with_a_visible_alias_fails_once_on_the_command_that_defines_it() {
    let shelf = Command::new("shelf")
        .arg(
            Arg::new("json")
                .long("json")
                .visible_alias("as-json")
                .global(true)
                .action(ArgAction::SetTrue),
        )
        .subcommand(Command::new("books").subcommand(Command::new("list")));

    assert_eq!(
        panic_message(|| check_names(&shelf, &Guide::new(), &[])).unwrap(),
        "the command tree breaks crumbkit's naming rules:\n\
         shelf: the argument `--json` has the visible alias `--as-json`, and help shows one name for each argument"
    );
}

#[test]
fn a_hidden_command_with_a_visible_alias_passes_because_no_help_page_shows_it() {
    let shelf = Command::new("shelf").subcommand(
        Command::new("books").subcommand(Command::new("purge").hide(true).visible_alias("wipe")),
    );

    check_names(&shelf, &Guide::new(), &[]);
}

#[test]
fn a_hidden_argument_with_a_visible_alias_passes_because_no_help_page_shows_it() {
    let list = Command::new("list").arg(
        Arg::new("legacy")
            .long("legacy")
            .hide(true)
            .visible_alias("old")
            .visible_short_alias('o')
            .action(ArgAction::SetTrue),
    );

    check_names(&list, &Guide::new(), &[]);
}

#[test]
fn a_visible_alias_passes_when_an_opt_out_names_its_command() {
    const ALLOW: &[Allow] = &[Allow::new(
        Rule::OneName,
        "books",
        "Scripts written before the rename call `shelf book`.",
    )];
    let shelf =
        Command::new("shelf").subcommand(Command::new("books").visible_alias("book").subcommand(
            Command::new("list").arg(Arg::new("limit").long("limit").visible_alias("max")),
        ));

    check_names(&shelf, &Guide::new(), ALLOW);
}

/// Names one alias, for a guide that shows a person's aliases on the root's
/// help.
fn names(_: &Command) -> Vec<String> {
    vec!["mine".to_owned()]
}

#[test]
fn a_guide_that_names_personal_aliases_on_the_root_s_help_fails_without_an_opt_out() {
    const GUIDE: Guide = Guide::new().personal_aliases(names);
    let shelf = Command::new("shelf").subcommand(Command::new("books"));

    assert_eq!(
        panic_message(|| check_names(&shelf, &GUIDE, &[])).unwrap(),
        "the command tree breaks crumbkit's naming rules:\n\
         shelf: the guide names a person's aliases on the root's help, and help shows one name for each command"
    );
}

#[test]
fn a_guide_that_names_personal_aliases_passes_with_an_opt_out_for_the_root() {
    const GUIDE: Guide = Guide::new().personal_aliases(names);
    const ALLOW: &[Allow] = &[Allow::new(
        Rule::PersonalAliases,
        "",
        "People who run shelf by hand forget the aliases they wrote.",
    )];
    let shelf = Command::new("shelf").subcommand(Command::new("books"));

    check_names(&shelf, &GUIDE, ALLOW);
}
