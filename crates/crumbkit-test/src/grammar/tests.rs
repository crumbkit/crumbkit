//! Tests for the grammar check.

use clap::{Arg, ArgAction, Command};

use super::check_grammar;
use crate::testing::panic_message;

#[test]
fn resources_that_nest_and_end_in_list_show_or_a_passed_verb_pass() {
    let shelf = Command::new("shelf")
        .subcommand(
            Command::new("books")
                .subcommand(Command::new("list"))
                .subcommand(Command::new("show"))
                .subcommand(Command::new("export"))
                .subcommand(
                    Command::new("reading-lists2")
                        .subcommand(Command::new("list"))
                        .subcommand(Command::new("export")),
                ),
        )
        .subcommand(Command::new("schema"));

    check_grammar(&shelf, &["export"], &["schema"]);
}

#[test]
fn a_command_without_subcommands_that_is_not_list_show_or_a_passed_verb_fails() {
    let shelf = Command::new("shelf").subcommand(
        Command::new("books")
            .subcommand(Command::new("list"))
            .subcommand(Command::new("remove")),
    );

    assert_eq!(
        panic_message(|| check_grammar(&shelf, &["export"], &[])).unwrap(),
        "the command tree breaks crumbkit's grammar:\n\
         shelf books remove: the command has no subcommands, so it is a verb, and a verb is `list`, `show` or one the tool passes"
    );
}

#[test]
fn a_command_without_subcommands_directly_under_the_tool_fails() {
    let shelf = Command::new("shelf")
        .subcommand(Command::new("books").subcommand(Command::new("list")))
        .subcommand(Command::new("schema"));

    assert_eq!(
        panic_message(|| check_grammar(&shelf, &[], &[])).unwrap(),
        "the command tree breaks crumbkit's grammar:\n\
         shelf schema: the command has no subcommands and sits directly under `shelf`, and a command has the form `shelf <resource> <verb>`"
    );
}

#[test]
fn a_resource_name_that_is_not_lowercase_words_joined_by_single_dashes_fails() {
    let mut shelf = Command::new("shelf");
    for name in [
        "Books",
        "reading_lists",
        "reading--lists",
        "-books",
        "2books",
    ] {
        shelf = shelf.subcommand(Command::new(name).subcommand(Command::new("list")));
    }

    assert_eq!(
        panic_message(|| check_grammar(&shelf, &[], &[])).unwrap(),
        "the command tree breaks crumbkit's grammar:\n\
         shelf Books: the resource name `Books` is not lowercase letters and digits in words joined by single dashes, starting with a letter\n\
         shelf reading_lists: the resource name `reading_lists` is not lowercase letters and digits in words joined by single dashes, starting with a letter\n\
         shelf reading--lists: the resource name `reading--lists` is not lowercase letters and digits in words joined by single dashes, starting with a letter\n\
         shelf -books: the resource name `-books` is not lowercase letters and digits in words joined by single dashes, starting with a letter\n\
         shelf 2books: the resource name `2books` is not lowercase letters and digits in words joined by single dashes, starting with a letter"
    );
}

#[test]
fn a_passed_verb_that_is_built_in_or_badly_formed_fails() {
    let shelf = Command::new("shelf");

    assert_eq!(
        panic_message(|| check_grammar(&shelf, &["show", "Export", ""], &[])).unwrap(),
        "the command tree breaks crumbkit's grammar:\n\
         the verb `show` is built in, so the tool does not pass it\n\
         the verb `Export` is not lowercase letters and digits in words joined by single dashes, starting with a letter\n\
         the verb `` is not lowercase letters and digits in words joined by single dashes, starting with a letter"
    );
}

#[test]
fn an_argument_named_dry_run_fails_anywhere_in_the_tree() {
    let dry_run = || {
        Arg::new("dry-run")
            .long("dry-run")
            .action(ArgAction::SetTrue)
    };
    let shelf = Command::new("shelf")
        .arg(dry_run())
        .subcommand(Command::new("books").subcommand(Command::new("list").arg(dry_run())));

    assert_eq!(
        panic_message(|| check_grammar(&shelf, &[], &[])).unwrap(),
        "the command tree breaks crumbkit's grammar:\n\
         shelf: the argument `--dry-run` uses a name that crumbkit reserves for its write verbs\n\
         shelf books list: the argument `--dry-run` uses a name that crumbkit reserves for its write verbs"
    );
}
