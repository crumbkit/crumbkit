//! Tests for runtime leaves and their join with a const guide.

use std::process::ExitCode;

use clap::Command;

use super::RuntimeLeaf;
use crate::help::{Example, Exit, Goal, Group, Guide, Leaf, RefusalContext, Rename, Topic};
use crate::{Call, Code, Sink, Spelling, parse_to};

const GUIDE: Guide = Guide::new()
    .groups(&[Group::new(&[], &[Goal::new("Read", &["planets"])])])
    .leaves(&[Leaf::new(&["planets", "list"]).examples(&[Example::new(&[], "List the planets")])])
    .topics(&[Topic::new(
        "orbits",
        "How the orbits work",
        "Planets orbit.\n",
    )])
    .exits(&[Exit::new(124, "The telescope ran out of time.")]);

#[test]
fn runtime_leaves_follow_the_const_leaves_with_their_paths_and_examples_in_order() {
    let guide = GUIDE.with_runtime_leaves(vec![
        RuntimeLeaf::new(vec!["moons".to_owned(), "list".to_owned()])
            .example(Vec::new(), "List the moons".to_owned())
            .example(vec!["--all".to_owned()], "List every moon".to_owned()),
        RuntimeLeaf::new(vec!["comets".to_owned()]),
    ]);

    let leaves: Vec<(&[&str], &[Example])> = guide
        .get_leaves()
        .iter()
        .map(|leaf| (leaf.path(), leaf.get_examples()))
        .collect();
    assert_eq!(
        leaves,
        [
            (
                ["planets", "list"].as_slice(),
                [Example::new(&[], "List the planets")].as_slice()
            ),
            (
                &["moons", "list"],
                &[
                    Example::new(&[], "List the moons"),
                    Example::new(&["--all"], "List every moon"),
                ]
            ),
            (&["comets"], &[]),
        ]
    );
    assert!(
        guide
            .get_leaves()
            .iter()
            .all(|leaf| leaf.get_document().is_none())
    );
}

#[test]
fn runtime_leaves_keep_the_groups_topics_and_exit_codes_of_the_const_guide() {
    let guide = GUIDE.with_runtime_leaves(Vec::new());

    assert_eq!(guide.get_groups(), GUIDE.get_groups());
    assert_eq!(guide.get_topics(), GUIDE.get_topics());
    assert_eq!(guide.get_exits(), GUIDE.get_exits());
    assert_eq!(guide.get_leaves().len(), 1);
}

#[test]
fn a_command_added_at_run_time_lists_the_examples_of_its_runtime_leaf_in_its_help() {
    let name = String::from("moons");
    let tool = Command::new("tool").subcommand(
        Command::new(name.clone()).subcommand(Command::new("list").about("List the moons")),
    );
    let guide = Guide::new().with_runtime_leaves(vec![
        RuntimeLeaf::new(vec![name, "list".to_owned()])
            .example(vec!["--all".to_owned()], "List every moon".to_owned()),
    ]);

    let mut out = Vec::new();
    let result = parse_to::<(), _, _, _, _>(
        &mut Sink::new(&mut out, Vec::new()),
        tool,
        &guide,
        Spelling::COLONS,
        ["tool", "moons", "list", "-h"],
    );

    assert_eq!(result.err().map(ExitCode::from), Some(ExitCode::SUCCESS));
    let page = String::from_utf8(out).unwrap();
    assert!(
        page.contains("Examples:\n  tool moons list --all\n"),
        "{page}"
    );
}

#[test]
fn a_runtime_leaf_keeps_the_program_whose_exit_code_it_passes_on() {
    let guide = Guide::new().with_runtime_leaves(vec![
        RuntimeLeaf::new(vec!["moons".to_owned(), "edit".to_owned()]).passes_on_exit("the editor"),
        RuntimeLeaf::new(vec!["moons".to_owned(), "list".to_owned()]),
    ]);

    let programs: Vec<Option<&str>> = guide
        .get_leaves()
        .iter()
        .map(Leaf::get_passes_on_exit)
        .collect();
    assert_eq!(programs, [Some("the editor"), None]);
}

/// The tool's own code for crumbkit's usage code.
const BAD_LINE: Code = Code::new("tool", "cli", "bad_line");

/// The guide's rename of crumbkit's usage code.
const RENAMES: &[Rename] = &[Rename::new(&crate::codes::CLI_USAGE, &BAD_LINE)];

/// Returns `tool moons list` for any refusal of a runtime leaf's command.
fn list_moons(_: &RefusalContext<'_>) -> Vec<Call> {
    vec![Call::new(&["moons", "list"])]
}

#[test]
fn a_refusal_of_a_runtime_leafs_command_recovers_with_its_functions_call_and_prints_the_rename() {
    let name = String::from("moons");
    let tool = Command::new("tool").subcommand(
        Command::new(name.clone())
            .subcommand(Command::new("list").about("List the moons"))
            .subcommand(Command::new("show").about("Show a moon")),
    );
    let guide = Guide::new().codes(RENAMES).with_runtime_leaves(vec![
        RuntimeLeaf::new(vec![name, "show".to_owned()]).refusal_alternatives(list_moons),
    ]);
    let mut err = Vec::new();

    let result = parse_to::<(), _, _, _, _>(
        &mut Sink::new(Vec::new(), &mut err),
        tool,
        &guide,
        Spelling::COLONS,
        ["tool", "moons", "show", "--bogus"],
    );

    assert_eq!(result.err().map(ExitCode::from), Some(ExitCode::from(2)));
    let text = String::from_utf8(err).unwrap();
    assert!(
        text.contains(
            "  help:  run `tool moons list`\n  help:  run `tool moons show -h`\n  code:  tool::cli::bad_line\n"
        ),
        "{text}"
    );
}
