//! Tests for the grammar check.

use clap::Command;
use crumbkit::help::Guide;

use super::check_grammar;
use crate::Pages;
use crate::allow::{Allow, Rule};
use crate::testing::panic_message;

#[test]
fn resources_that_nest_and_end_in_list_show_or_a_passed_verb_pass() {
    let shelf = Command::new("shelf").subcommand(
        Command::new("books")
            .subcommand(Command::new("list"))
            .subcommand(Command::new("show"))
            .subcommand(Command::new("export"))
            .subcommand(
                Command::new("reading-lists2")
                    .subcommand(Command::new("list"))
                    .subcommand(Command::new("export")),
            ),
    );

    check_grammar(&shelf, Pages::clap(), &["export"], &[]);
}

#[test]
fn a_command_without_subcommands_that_is_not_list_show_or_a_passed_verb_fails() {
    let shelf = Command::new("shelf").subcommand(
        Command::new("books")
            .subcommand(Command::new("list"))
            .subcommand(Command::new("remove")),
    );

    assert_eq!(
        panic_message(|| check_grammar(&shelf, Pages::clap(), &["export"], &[])).unwrap(),
        "the command tree breaks crumbkit's grammar:\n\
         shelf books remove: the command has no subcommands, so it is a verb, but `remove` is not `list`, `show` or a verb the tool passes"
    );
}

#[test]
fn a_command_without_subcommands_directly_under_the_tool_fails() {
    let shelf = Command::new("shelf")
        .subcommand(Command::new("books").subcommand(Command::new("list")))
        .subcommand(Command::new("schema"));

    assert_eq!(
        panic_message(|| check_grammar(&shelf, Pages::clap(), &[], &[])).unwrap(),
        "the command tree breaks crumbkit's grammar:\n\
         shelf schema: the command has no subcommands and sits directly under `shelf`, and a command has the form `shelf <resource> <verb>`"
    );
}

/// A guide that declares dashed forms.
const DASHED: Guide = Guide::new().dashed_forms();

#[test]
fn a_resource_name_that_is_not_lowercase_words_joined_by_single_dashes_fails_without_dashed_forms()
{
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
        panic_message(|| check_grammar(&shelf, Pages::clap(), &[], &[])).unwrap(),
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
        panic_message(|| check_grammar(&shelf, Pages::clap(), &["show", "Export", ""], &[]))
            .unwrap(),
        "the command tree breaks crumbkit's grammar:\n\
         the tool passes the verb `show`, which is built in, and a tool passes only its other verbs\n\
         the verb `Export` is not lowercase letters and digits in words joined by single dashes, starting with a letter\n\
         the verb `` is not lowercase letters and digits in words joined by single dashes, starting with a letter"
    );
}

#[test]
fn a_resource_or_verb_named_in_another_style_passes_when_the_guide_declares_dashed_forms() {
    let shelf = Command::new("shelf")
        .subcommand(Command::new("Books").subcommand(Command::new("export_all")))
        .subcommand(Command::new("reading_lists").subcommand(Command::new("list")))
        .subcommand(Command::new("AuthorNotes").subcommand(Command::new("list")));

    check_grammar(&shelf, Pages::rendered(&DASHED), &["export_all"], &[]);
}

#[test]
fn with_dashed_forms_a_name_with_no_dashed_form_or_whose_dashed_form_another_command_takes_fails() {
    let list = || Command::new("list");
    let mut shelf = Command::new("shelf");
    for name in ["2books", "Authors", "authors", "Help"] {
        shelf = shelf.subcommand(Command::new(name).subcommand(list()));
    }
    shelf = shelf.subcommand(
        Command::new("books")
            .subcommand(Command::new("export_all"))
            .subcommand(Command::new("export-all")),
    );
    let verbs = ["export_all", "export-all", "2export"];

    assert_eq!(
        panic_message(|| check_grammar(&shelf, Pages::rendered(&DASHED), &verbs, &[])).unwrap(),
        "the command tree breaks crumbkit's grammar:\n\
         the verb `2export` is not lowercase letters and digits in words joined by single dashes, starting with a letter, and maps to no such name\n\
         shelf 2books: the resource name `2books` is not lowercase letters and digits in words joined by single dashes, starting with a letter, and maps to no such name\n\
         shelf Authors: the resource name `Authors` maps to `authors`, which another command beside it has or maps to\n\
         shelf Help: the resource name `Help` maps to `help`, which crumbkit reserves for asking for help\n\
         shelf books export_all: the verb name `export_all` maps to `export-all`, which another command beside it has or maps to"
    );
}

#[test]
fn a_command_below_the_one_passed_in_named_or_aliased_help_fails_the_check() {
    let list = || Command::new("list");
    let shelf = Command::new("shelf")
        .subcommand(Command::new("help").subcommand(list()))
        .subcommand(
            Command::new("books")
                .subcommand(list())
                .subcommand(Command::new("guides").alias("help").subcommand(list())),
        );

    assert_eq!(
        panic_message(|| check_grammar(&shelf, Pages::clap(), &[], &[])).unwrap(),
        "the command tree breaks crumbkit's grammar:\n\
         shelf help: the command answers to `help`, and crumbkit reserves that word for asking for help\n\
         shelf books guides: the command answers to `help`, and crumbkit reserves that word for asking for help"
    );
}

#[test]
fn the_command_passed_in_may_be_named_or_aliased_help() {
    let help = Command::new("help")
        .alias("help")
        .subcommand(Command::new("books").subcommand(Command::new("list")));

    check_grammar(&help, Pages::clap(), &[], &[]);
}

#[test]
fn a_leaf_directly_under_the_tool_passes_when_an_opt_out_names_it_and_its_other_rules_still_apply()
{
    const ALLOW: &[Allow] = &[Allow::new(
        Rule::RootLeaf,
        "version",
        "People type `shelf version` from habit.",
    )];
    let shelf = Command::new("shelf")
        .subcommand(Command::new("books").subcommand(Command::new("list")))
        .subcommand(Command::new("version").alias("help"));

    assert_eq!(
        panic_message(|| check_grammar(&shelf, Pages::clap(), &[], ALLOW)).unwrap(),
        "the command tree breaks crumbkit's grammar:\n\
         shelf version: the command answers to `help`, and crumbkit reserves that word for asking for help"
    );
}

#[test]
fn a_resource_keeps_its_name_when_an_opt_out_names_it() {
    const ALLOW: &[Allow] = &[Allow::new(
        Rule::ResourceName,
        "reading_lists",
        "The name matches the service's own API.",
    )];
    let shelf = Command::new("shelf")
        .subcommand(Command::new("reading_lists").subcommand(Command::new("list")));

    check_grammar(&shelf, Pages::clap(), &[], ALLOW);
}

#[test]
fn an_opt_out_from_a_grammar_rule_that_excuses_nothing_fails_the_check() {
    const ALLOW: &[Allow] = &[Allow::new(Rule::RootLeaf, "books", "Old habit.")];
    let shelf =
        Command::new("shelf").subcommand(Command::new("books").subcommand(Command::new("list")));

    assert_eq!(
        panic_message(|| check_grammar(&shelf, Pages::clap(), &[], ALLOW)).unwrap(),
        "the command tree breaks crumbkit's grammar:\n\
         the opt-out from `grammar/root-leaf` for `shelf books` excuses no problem, so it is stale"
    );
}

#[test]
fn a_hidden_leaf_named_query_with_leading_underscores_passes_the_grammar_check() {
    let shelf = Command::new("shelf").subcommand(
        Command::new("books")
            .subcommand(Command::new("list"))
            .subcommand(Command::new("__query").hide(true)),
    );

    check_grammar(&shelf, Pages::clap(), &[], &[]);
}

#[test]
fn a_hidden_command_and_everything_below_it_are_outside_the_grammar() {
    let shelf = Command::new("shelf")
        .subcommand(Command::new("books").subcommand(Command::new("list")))
        .subcommand(Command::new("__complete").hide(true))
        .subcommand(
            Command::new("Internal")
                .hide(true)
                .subcommand(Command::new("rebuild_index")),
        );

    check_grammar(&shelf, Pages::clap(), &[], &[]);
}

#[test]
fn a_hidden_command_named_or_aliased_help_still_fails_the_check() {
    let shelf = Command::new("shelf").subcommand(
        Command::new("books")
            .subcommand(Command::new("list"))
            .subcommand(Command::new("help").hide(true))
            .subcommand(Command::new("__guides").alias("help").hide(true)),
    );

    assert_eq!(
        panic_message(|| check_grammar(&shelf, Pages::clap(), &[], &[])).unwrap(),
        "the command tree breaks crumbkit's grammar:\n\
         shelf books help: the command answers to `help`, and crumbkit reserves that word for asking for help\n\
         shelf books __guides: the command answers to `help`, and crumbkit reserves that word for asking for help"
    );
}

#[test]
fn an_opt_out_from_a_grammar_rule_for_a_hidden_command_is_stale() {
    const ALLOW: &[Allow] = &[Allow::new(
        Rule::RootLeaf,
        "__complete",
        "Shells call it for completions.",
    )];
    let shelf = Command::new("shelf")
        .subcommand(Command::new("books").subcommand(Command::new("list")))
        .subcommand(Command::new("__complete").hide(true));

    assert_eq!(
        panic_message(|| check_grammar(&shelf, Pages::clap(), &[], ALLOW)).unwrap(),
        "the command tree breaks crumbkit's grammar:\n\
         the opt-out from `grammar/root-leaf` for `shelf __complete` excuses no problem, so it is stale"
    );
}
