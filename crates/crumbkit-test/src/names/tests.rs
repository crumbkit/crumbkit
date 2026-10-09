//! Tests for the names check.

use clap::{Arg, ArgAction, Command};

use super::check_names;
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

    check_names(&shelf, &[]);
}

#[test]
fn a_command_with_a_visible_or_a_hidden_alias_fails() {
    let shelf = Command::new("shelf").subcommand(
        Command::new("books")
            .visible_alias("book")
            .alias("b")
            .subcommand(Command::new("list")),
    );

    assert_eq!(
        panic_message(|| check_names(&shelf, &[])).unwrap(),
        "the command tree breaks crumbkit's naming rules:\n\
         shelf books: the command has the alias `book`, and a command has no aliases\n\
         shelf books: the command has the alias `b`, and a command has no aliases"
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
        panic_message(|| check_names(&shelf, &[])).unwrap(),
        "the command tree breaks crumbkit's naming rules:\n\
         shelf books: the command has the short flag name `-B`, and a command has one name\n\
         shelf books: the command has the long flag name `--books`, and a command has one name"
    );
}

#[test]
fn a_command_with_a_short_or_a_long_flag_alias_fails() {
    let shelf = Command::new("shelf").subcommand(
        Command::new("books")
            .short_flag_alias('k')
            .long_flag_alias("book"),
    );

    assert_eq!(
        panic_message(|| check_names(&shelf, &[])).unwrap(),
        "the command tree breaks crumbkit's naming rules:\n\
         shelf books: the command has the short flag alias `-k`, and a command has no aliases\n\
         shelf books: the command has the long flag alias `--book`, and a command has no aliases"
    );
}

#[test]
fn an_argument_with_a_long_or_a_short_alias_visible_or_hidden_fails() {
    let list = Command::new("list").arg(
        Arg::new("limit")
            .long("limit")
            .short('l')
            .visible_alias("max")
            .short_alias('n')
            .action(ArgAction::Set),
    );

    assert_eq!(
        panic_message(|| check_names(&list, &[])).unwrap(),
        "the command tree breaks crumbkit's naming rules:\n\
         list: the argument `--limit` has the alias `--max`, and an argument has no aliases\n\
         list: the argument `--limit` has the short alias `-n`, and an argument has no aliases"
    );
}

#[test]
fn a_global_argument_with_an_alias_fails_once_on_the_command_that_defines_it() {
    let shelf = Command::new("shelf")
        .arg(
            Arg::new("json")
                .long("json")
                .alias("as-json")
                .global(true)
                .action(ArgAction::SetTrue),
        )
        .subcommand(Command::new("books").subcommand(Command::new("list")));

    assert_eq!(
        panic_message(|| check_names(&shelf, &[])).unwrap(),
        "the command tree breaks crumbkit's naming rules:\n\
         shelf: the argument `--json` has the alias `--as-json`, and an argument has no aliases"
    );
}
