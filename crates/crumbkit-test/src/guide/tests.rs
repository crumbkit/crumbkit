//! Tests for the guide check.

use clap::Command;
use crumbkit::help::{Goal, Group, Guide, Topic};

use super::check_guide;
use crate::testing::panic_message;

/// A tool with two resources, one of which has a hidden verb and one an
/// alias.
fn shelf() -> Command {
    Command::new("shelf")
        .subcommand(
            Command::new("books")
                .alias("titles")
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
        "the command tree breaks crumbkit's guide rules:\n\
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
        "the command tree breaks crumbkit's guide rules:\n\
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
        "the command tree breaks crumbkit's guide rules:\n\
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
        "the command tree breaks crumbkit's guide rules:\n\
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
        "the command tree breaks crumbkit's guide rules:\n\
         shelf books: the goals `Read` and `Check` name `show`, so help lists it under each of them"
    );
}

/// A topic that passes every topic rule.
const CATALOGUE: Topic = Topic::new(
    "catalogue",
    "Where the books come from",
    "The catalogue is built in.\n",
);

#[test]
fn a_guide_with_well_formed_topics_passes() {
    const GUIDE: Guide = Guide::new().topics(&[
        CATALOGUE,
        Topic::new("book-ids2", "", "Ids run from 1 to 6."),
    ]);

    check_guide(&shelf(), &GUIDE);
}

#[test]
fn a_topic_named_like_a_root_command_or_one_of_its_aliases_fails_the_check() {
    const GUIDE: Guide = Guide::new().topics(&[
        Topic::new("books", "", "Books.\n"),
        Topic::new("titles", "", "Titles.\n"),
    ]);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE)).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         the topic `books` is a name of the command `shelf books`, so `shelf help books` prints that command's help\n\
         the topic `titles` is a name of the command `shelf books`, so `shelf help titles` prints that command's help"
    );
}

#[test]
fn a_topic_whose_text_is_empty_fails_the_check() {
    const GUIDE: Guide = Guide::new().topics(&[
        Topic::new("catalogue", "", ""),
        Topic::new("ids", "", " \n\t"),
    ]);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE)).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         the topic `catalogue` has no text\n\
         the topic `ids` has no text"
    );
}

#[test]
fn two_topics_with_one_name_fail_the_check() {
    const GUIDE: Guide = Guide::new().topics(&[CATALOGUE, CATALOGUE]);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE)).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         two topics are named `catalogue`, and `shelf help catalogue` prints only the first"
    );
}

#[test]
fn a_topic_name_that_is_not_lowercase_words_joined_by_dashes_fails_the_check() {
    const GUIDE: Guide = Guide::new().topics(&[
        Topic::new("Catalogue", "", "The catalogue.\n"),
        Topic::new("book_ids", "", "The ids.\n"),
    ]);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE)).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         the topic name `Catalogue` is not lowercase letters and digits in words joined by single dashes, starting with a letter\n\
         the topic name `book_ids` is not lowercase letters and digits in words joined by single dashes, starting with a letter"
    );
}
