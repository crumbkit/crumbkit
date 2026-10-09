//! Tests for the guide check.

use clap::Command;
use crumbkit::help::{Goal, Group, Guide};

use super::check_guide;
use crate::testing::panic_message;

/// A tool with two resources, one of which has a hidden verb.
fn shelf() -> Command {
    Command::new("shelf")
        .subcommand(
            Command::new("books")
                .subcommand(Command::new("list"))
                .subcommand(Command::new("show"))
                .subcommand(Command::new("purge").hide(true)),
        )
        .subcommand(Command::new("authors").subcommand(Command::new("list")))
}

/// The root's goals, which name both resources.
const ROOT: Group = Group::new(&[], &[Goal::new("Browse", &["books", "authors"])]);

#[test]
fn a_guide_that_names_every_visible_child_of_each_group_passes() {
    const GUIDE: Guide = Guide::new().groups(&[
        ROOT,
        Group::new(
            &["books"],
            &[
                Goal::new("Read", &["list", "show"]),
                Goal::new("Change", &["purge"]),
            ],
        ),
    ]);

    check_guide(&shelf(), &GUIDE);
}

#[test]
fn a_hidden_child_that_no_goal_names_passes() {
    const GUIDE: Guide = Guide::new().groups(&[Group::new(
        &["books"],
        &[Goal::new("Read", &["list", "show"])],
    )]);

    check_guide(&shelf(), &GUIDE);
}

#[test]
fn a_visible_child_that_no_goal_names_fails_the_check() {
    const GUIDE: Guide =
        Guide::new().groups(&[Group::new(&["books"], &[Goal::new("Read", &["list"])])]);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE)).unwrap(),
        "the command tree breaks crumbkit's goal heading rules:\n\
         shelf books: no goal names the visible subcommand `show`, so help lists it under a final `Commands` heading"
    );
}

#[test]
fn a_goal_name_that_its_group_lacks_fails_the_check() {
    const GUIDE: Guide = Guide::new().groups(&[Group::new(
        &["books"],
        &[Goal::new("Read", &["list", "show", "lst"])],
    )]);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE)).unwrap(),
        "the command tree breaks crumbkit's goal heading rules:\n\
         shelf books: the goal `Read` names `lst`, which is not a subcommand of the command"
    );
}

#[test]
fn a_group_path_that_names_no_command_fails_the_check() {
    const GUIDE: Guide = Guide::new().groups(&[
        ROOT,
        Group::new(&["shelves"], &[Goal::new("Read", &["list"])]),
    ]);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE)).unwrap(),
        "the command tree breaks crumbkit's goal heading rules:\n\
         the group `shelves` names no command"
    );
}

#[test]
fn a_group_path_that_names_a_command_without_subcommands_fails_the_check() {
    const GUIDE: Guide = Guide::new().groups(&[Group::new(
        &["books", "list"],
        &[Goal::new("Read", &["list"])],
    )]);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE)).unwrap(),
        "the command tree breaks crumbkit's goal heading rules:\n\
         shelf books list: the guide gives the command goals, and it has no subcommands to list under them"
    );
}

#[test]
fn a_command_that_two_goals_name_fails_the_check() {
    const GUIDE: Guide = Guide::new().groups(&[Group::new(
        &["books"],
        &[
            Goal::new("Read", &["list", "show"]),
            Goal::new("Check", &["show"]),
        ],
    )]);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE)).unwrap(),
        "the command tree breaks crumbkit's goal heading rules:\n\
         shelf books: the goals `Read` and `Check` name `show`, so help lists it under each of them"
    );
}
