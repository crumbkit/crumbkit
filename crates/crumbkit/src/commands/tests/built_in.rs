//! Tests for the `built_in` field, which marks the rows of the commands
//! that crumbkit or one of its add-on crates provides.

use clap::{Args as _, Command};
use serde_json::Value;

use super::{Book, row};
use crate::commands::{COMMANDS_LIST, CommandsList, list};
use crate::help::{Help, Leaf};
use crate::{Document, Run, Tool};

/// A document that an add-on crate marks as its own, as crumbkit-usage
/// marks `self usage`'s.
const ADD_ON: Document<Vec<Book>> =
    Document::new("crumbkit", "self-usage", 1, "The add-on's rows.").built_in();

/// A document of the tool's own.
const DOCTOR: Document<Vec<Book>> = Document::new("shelf", "self-doctor", 1, "The checks.");

/// A tool record with leaves for `commands list` and `self usage`, whose
/// documents crumbkit marks, for the tool's own `self doctor` and
/// `books list`, and for nothing under `self figures`.
const TOOL: Tool = Tool::new().help(Help::new().leaves(&[
    Leaf::new(&["commands", "list"]).document(COMMANDS_LIST),
    Leaf::new(&["self", "usage"]).document(ADD_ON),
    Leaf::new(&["self", "doctor"]).document(DOCTOR),
    Leaf::new(&["books", "list"]).document(DOCTOR),
]));

/// A tool with `commands list`, a `self` group that holds `usage` and the
/// tool's own `doctor`, hidden when `doctor_hidden`, a `books` group whose
/// only subcommand is hidden, and a `notes` group with `figures`, which
/// has no leaf.
fn tree(doctor_hidden: bool) -> Command {
    Command::new("shelf")
        .disable_help_subcommand(true)
        .subcommand(
            Command::new("commands").subcommand(CommandsList::augment_args(Command::new("list"))),
        )
        .subcommand(
            Command::new("self")
                .subcommand(Command::new("usage"))
                .subcommand(Command::new("doctor").hide(doctor_hidden)),
        )
        .subcommand(Command::new("books").subcommand(Command::new("list").hide(true)))
        .subcommand(Command::new("notes").subcommand(Command::new("figures")))
}

/// Lists the whole tree with `doctor_hidden` and returns the rows as JSON.
fn rows(doctor_hidden: bool) -> Value {
    let command = tree(doctor_hidden);
    let mut matches = command.clone();
    let matches = matches
        .try_get_matches_from_mut(["shelf", "commands", "list"])
        .unwrap();
    let (_, matches) = matches.subcommand().unwrap();
    let (_, matches) = matches.subcommand().unwrap();
    let args = <CommandsList as clap::FromArgMatches>::from_arg_matches(matches).unwrap();
    serde_json::to_value(list(&mut Run::of(command), &TOOL, &args).unwrap()).unwrap()
}

/// Returns the `built_in` of the row whose path is `path` among `rows`.
fn built_in<'a>(rows: &'a Value, path: &str) -> &'a Value {
    &row(rows, path)["built_in"]
}

#[test]
fn a_command_whose_leaf_declares_a_document_that_crumbkit_marks_is_built_in() {
    let rows = rows(false);

    assert_eq!(built_in(&rows, "shelf commands list"), &Value::Bool(true));
    assert_eq!(built_in(&rows, "shelf self usage"), &Value::Bool(true));
}

#[test]
fn the_tool_s_own_command_under_a_crumbkit_group_is_not_built_in() {
    let rows = rows(false);

    assert_eq!(built_in(&rows, "shelf self doctor"), &Value::Bool(false));
}

#[test]
fn a_group_is_built_in_only_when_every_visible_command_below_it_is() {
    assert_eq!(built_in(&rows(false), "shelf commands"), &Value::Bool(true));
    assert_eq!(built_in(&rows(false), "shelf self"), &Value::Bool(false));
    assert_eq!(built_in(&rows(true), "shelf self"), &Value::Bool(true));
}

#[test]
fn a_group_without_a_visible_subcommand_and_a_command_without_a_leaf_are_not_built_in() {
    let rows = rows(false);

    assert_eq!(built_in(&rows, "shelf books"), &Value::Bool(false));
    assert_eq!(built_in(&rows, "shelf notes figures"), &Value::Bool(false));
    assert_eq!(built_in(&rows, "shelf notes"), &Value::Bool(false));
    assert_eq!(built_in(&rows, "shelf"), &Value::Bool(false));
}

#[test]
fn a_branch_marks_its_first_row_from_every_visible_command_below_it() {
    let command = tree(true);
    let mut parsing = command.clone();
    let matches = parsing
        .try_get_matches_from_mut(["shelf", "commands", "list", "self", "--limit", "1"])
        .unwrap();
    let (_, matches) = matches.subcommand().unwrap();
    let (_, matches) = matches.subcommand().unwrap();
    let args = <CommandsList as clap::FromArgMatches>::from_arg_matches(matches).unwrap();
    let rows = serde_json::to_value(list(&mut Run::of(command), &TOOL, &args).unwrap()).unwrap();

    assert_eq!(built_in(&rows, "shelf self"), &Value::Bool(true));
}

#[test]
fn commands_list_and_the_schemas_documents_are_marked_as_crumbkit_s_own() {
    assert!(COMMANDS_LIST.is_built_in());
    assert!(crate::schemas::SCHEMAS_LIST.is_built_in());
    assert!(crate::schemas::SCHEMAS_SHOW.is_built_in());
}
