//! Tests for the guide check.

use clap::{Arg, ArgAction, Command};
use crumbkit::help::{Example, Goal, Group, Guide, Leaf, Topic};
use crumbkit::{Document, Row};
use schemars::JsonSchema;
use serde::Serialize;

use super::check_guide;
use crate::testing::panic_message;

mod documents;
mod examples;
mod lines;
mod problems;
mod uncut;
mod views;

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

/// One example that runs a command without arguments.
const RUN: &[Example] = &[Example::new(&[], "Run the command")];

/// Returns the leaf of the command at `path` with one example that runs it
/// without arguments.
const fn leaf(path: &'static [&'static str]) -> Leaf {
    Leaf::new(path).examples(RUN)
}

/// A leaf with an example for each visible command without subcommands of
/// `shelf()`, so a test whose guide holds them reports only its own
/// problem.
const LEAVES: &[Leaf] = &[
    leaf(&["books", "list"]),
    leaf(&["books", "show"]),
    leaf(&["authors", "list"]),
];

#[test]
fn a_guide_that_names_every_visible_child_of_each_group_passes() {
    const GUIDE: Guide = Guide::new()
        .groups(&[
            ROOT,
            Group::new(
                &["books"],
                &[
                    Goal::new("Read", &["list", "show"]),
                    Goal::new("Change", &["purge"]),
                ],
            ),
        ])
        .leaves(LEAVES);

    check_guide(&shelf(), &GUIDE, &[]);
}

#[test]
fn a_hidden_child_that_no_goal_names_passes() {
    const GUIDE: Guide = Guide::new()
        .groups(&[Group::new(
            &["books"],
            &[Goal::new("Read", &["list", "show"])],
        )])
        .leaves(LEAVES);

    check_guide(&shelf(), &GUIDE, &[]);
}

#[test]
fn a_visible_child_that_no_goal_names_fails_the_check() {
    const GUIDE: Guide = Guide::new()
        .groups(&[Group::new(&["books"], &[Goal::new("Read", &["list"])])])
        .leaves(LEAVES);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE, &[])).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         shelf books: no goal names the visible subcommand `show`, so help lists it under a final `Commands` heading"
    );
}

#[test]
fn a_goal_name_that_its_group_lacks_fails_the_check() {
    const GUIDE: Guide = Guide::new()
        .groups(&[Group::new(
            &["books"],
            &[Goal::new("Read", &["list", "show", "lst"])],
        )])
        .leaves(LEAVES);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE, &[])).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         shelf books: the goal `Read` names `lst`, which is not a subcommand of the command"
    );
}

#[test]
fn a_group_path_that_names_no_command_fails_the_check() {
    const GUIDE: Guide = Guide::new()
        .groups(&[
            ROOT,
            Group::new(&["shelves"], &[Goal::new("Read", &["list"])]),
        ])
        .leaves(LEAVES);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE, &[])).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         the group `shelves` names no command"
    );
}

#[test]
fn a_root_path_that_names_no_command_fails_the_check() {
    const GUIDE: Guide = Guide::new().root(&["shelves"]).leaves(LEAVES);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE, &[])).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         the root `shelves` names no command"
    );
}

#[test]
fn a_group_path_that_names_a_command_without_subcommands_fails_the_check() {
    const GUIDE: Guide = Guide::new()
        .groups(&[Group::new(
            &["books", "list"],
            &[Goal::new("Read", &["list"])],
        )])
        .leaves(LEAVES);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE, &[])).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         shelf books list: the guide gives the command goals, and it has no subcommands to list under them"
    );
}

#[test]
fn a_command_that_two_goals_name_fails_the_check() {
    const GUIDE: Guide = Guide::new()
        .groups(&[Group::new(
            &["books"],
            &[
                Goal::new("Read", &["list", "show"]),
                Goal::new("Check", &["show"]),
            ],
        )])
        .leaves(LEAVES);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE, &[])).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         shelf books: the goals `Read` and `Check` name `show`, so help lists it under each of them"
    );
}

#[test]
fn a_command_that_one_goal_names_twice_fails_the_check() {
    const GUIDE: Guide = Guide::new()
        .groups(&[Group::new(
            &["books"],
            &[Goal::new("Read", &["list", "show", "show"])],
        )])
        .leaves(LEAVES);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE, &[])).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         shelf books: the goal `Read` names `show` more than once, so help lists it more than once under that heading"
    );
}

#[test]
fn two_groups_with_one_path_fail_the_check() {
    const BOOKS: Group = Group::new(&["books"], &[Goal::new("Read", &["list", "show"])]);
    const GUIDE: Guide = Guide::new().groups(&[BOOKS, BOOKS, BOOKS]).leaves(LEAVES);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE, &[])).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         shelf books: two groups name the command, and help shows only the goals of the last"
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
    const GUIDE: Guide = Guide::new().leaves(LEAVES).topics(&[
        CATALOGUE,
        Topic::new("book-ids2", "", "Ids run from 1 to 6."),
    ]);

    check_guide(&shelf(), &GUIDE, &[]);
}

#[test]
fn a_topic_named_like_a_root_command_or_one_of_its_aliases_fails_the_check() {
    const GUIDE: Guide = Guide::new().leaves(LEAVES).topics(&[
        Topic::new("books", "", "Books.\n"),
        Topic::new("titles", "", "Titles.\n"),
    ]);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE, &[])).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         the topic `books` is a name of the command `shelf books`, so `shelf help books` prints that command's help\n\
         the topic `titles` is a name of the command `shelf books`, so `shelf help titles` prints that command's help"
    );
}

#[test]
fn a_topic_whose_text_is_empty_fails_the_check() {
    const GUIDE: Guide = Guide::new().leaves(LEAVES).topics(&[
        Topic::new("catalogue", "", ""),
        Topic::new("ids", "", " \n\t"),
    ]);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE, &[])).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         the topic `catalogue` has no text\n\
         the topic `ids` has no text"
    );
}

#[test]
fn two_topics_with_one_name_fail_the_check() {
    const GUIDE: Guide = Guide::new().leaves(LEAVES).topics(&[CATALOGUE, CATALOGUE]);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE, &[])).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         two topics are named `catalogue`, and `shelf help catalogue` prints only the first"
    );
}

#[test]
fn a_topic_named_like_crumbkit_s_exit_codes_topic_fails_the_check() {
    const GUIDE: Guide =
        Guide::new()
            .leaves(LEAVES)
            .topics(&[Topic::new("exit-codes", "", "Codes.\n")]);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE, &[])).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         the topic `exit-codes` has the name of crumbkit's own topic, so `shelf help exit-codes` prints crumbkit's exit codes in its place"
    );
}

#[test]
fn a_topic_name_that_is_not_lowercase_words_joined_by_dashes_fails_the_check() {
    const GUIDE: Guide = Guide::new().leaves(LEAVES).topics(&[
        Topic::new("Catalogue", "", "The catalogue.\n"),
        Topic::new("book_ids", "", "The ids.\n"),
    ]);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE, &[])).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         the topic name `Catalogue` is not lowercase letters and digits in words joined by single dashes, starting with a letter\n\
         the topic name `book_ids` is not lowercase letters and digits in words joined by single dashes, starting with a letter"
    );
}

/// A tool whose `books list` takes `--all`, whose `books show` takes ids,
/// and whose every command has `--version`.
fn shelf_with_arguments() -> Command {
    Command::new("shelf")
        .version("1.0.0")
        .propagate_version(true)
        .subcommand(
            Command::new("books")
                .subcommand(
                    Command::new("list")
                        .arg(Arg::new("all").long("all").action(ArgAction::SetTrue)),
                )
                .subcommand(Command::new("show").arg(Arg::new("ids").required(true).num_args(1..))),
        )
}

#[test]
fn a_guide_whose_leaves_name_commands_without_subcommands_and_examples_that_parse_passes() {
    const GUIDE: Guide = Guide::new().leaves(&[
        Leaf::new(&["books", "list"]).examples(&[
            Example::new(&[], "Print the first page"),
            Example::new(&["--all"], "Print every book"),
        ]),
        Leaf::new(&["books", "show"]).examples(&[Example::new(&["2", "5"], "Show two books")]),
    ]);

    check_guide(&shelf_with_arguments(), &GUIDE, &[]);
}

#[test]
fn a_leaf_path_that_names_no_command_fails_the_check() {
    const GUIDE: Guide = Guide::new().leaves(&[
        leaf(&["books", "list"]),
        leaf(&["books", "show"]),
        leaf(&["authors", "list"]),
        Leaf::new(&["books", "lst"]),
    ]);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE, &[])).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         the leaf `books lst` names no command"
    );
}

#[test]
fn a_leaf_path_that_names_a_command_with_subcommands_fails_the_check() {
    const GUIDE: Guide = Guide::new().leaves(&[
        leaf(&["books", "list"]),
        leaf(&["books", "show"]),
        leaf(&["authors", "list"]),
        Leaf::new(&["books"]),
    ]);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE, &[])).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         shelf books: the guide declares the command a leaf, and it has subcommands"
    );
}

#[test]
fn two_leaves_with_one_path_fail_the_check() {
    const GUIDE: Guide = Guide::new().leaves(&[
        leaf(&["books", "list"]),
        leaf(&["books", "list"]),
        leaf(&["books", "show"]),
        leaf(&["authors", "list"]),
    ]);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE, &[])).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         shelf books list: two leaves name the command, and help shows only the first"
    );
}

#[test]
fn an_example_that_clap_refuses_fails_the_check_naming_its_command_line_as_help_prints_it() {
    const GUIDE: Guide = Guide::new().leaves(&[
        Leaf::new(&["books", "list"])
            .examples(&[Example::new(&["--al", "Jane Austen"], "Print every book")]),
        Leaf::new(&["books", "show"]).examples(&[Example::new(&[], "Show no books")]),
    ]);

    assert_eq!(
        panic_message(|| check_guide(&shelf_with_arguments(), &GUIDE, &[])).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         shelf books list: clap refuses the example `shelf books list --al 'Jane Austen'` with the error kind UnknownArgument\n\
         shelf books show: clap refuses the example `shelf books show` with the error kind MissingRequiredArgument"
    );
}

#[test]
fn an_example_that_asks_for_help_or_the_version_fails_the_check() {
    const GUIDE: Guide = Guide::new().leaves(&[
        Leaf::new(&["books", "list"]).examples(&[
            Example::new(&["--help"], "Print help"),
            Example::new(&["--version"], "Print the version"),
        ]),
        Leaf::new(&["books", "show"]).examples(&[Example::new(&["2"], "Show one book")]),
    ]);

    assert_eq!(
        panic_message(|| check_guide(&shelf_with_arguments(), &GUIDE, &[])).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         shelf books list: clap refuses the example `shelf books list --help` with the error kind DisplayHelp\n\
         shelf books list: clap refuses the example `shelf books list --version` with the error kind DisplayVersion"
    );
}

#[test]
fn a_visible_command_without_subcommands_that_no_leaf_with_an_example_names_fails_the_check() {
    const GUIDE: Guide =
        Guide::new().leaves(&[leaf(&["books", "list"]), Leaf::new(&["books", "show"])]);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE, &[])).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         shelf books show: no leaf with an example names the command, so its help shows no example\n\
         shelf authors list: no leaf with an example names the command, so its help shows no example"
    );
}

#[test]
fn a_root_without_subcommands_needs_a_leaf_with_an_example() {
    const GUIDE: Guide = Guide::new();
    const ROOT_LEAF: Guide = Guide::new().leaves(&[leaf(&[])]);
    let tool = Command::new("stamp");

    assert_eq!(
        panic_message(|| check_guide(&tool, &GUIDE, &[])).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         stamp: no leaf with an example names the command, so its help shows no example"
    );
    check_guide(&tool, &ROOT_LEAF, &[]);
}

/// A book whose every column is a documented field.
#[derive(Serialize, JsonSchema)]
struct Book {
    /// The book's id.
    id: u32,
    /// The book's title.
    title: String,
    /// The number of pages.
    pages: u32,
}

impl Row for Book {
    const COLUMNS: &'static [&'static str] = &["id", "title"];
    const LONG_COLUMNS: &'static [&'static str] = &["pages"];
}

/// A book whose columns misspell `title` and `pages`.
#[derive(Serialize, JsonSchema)]
struct MisspeltBook {
    /// The book's id.
    id: u32,
    /// The book's title.
    title: String,
    /// The number of pages.
    pages: u32,
}

impl Row for MisspeltBook {
    const COLUMNS: &'static [&'static str] = &["id", "titel"];
    const LONG_COLUMNS: &'static [&'static str] = &["page"];
}

/// A book whose `title` and `pages` fields have no doc comment.
#[derive(Serialize, JsonSchema)]
struct UndocumentedBook {
    /// The book's id.
    id: u32,
    title: String,
    pages: u32,
}

impl Row for UndocumentedBook {
    const COLUMNS: &'static [&'static str] = &["id", "title"];
    const LONG_COLUMNS: &'static [&'static str] = &["pages"];
}

#[test]
fn a_leaf_whose_columns_are_documented_fields_of_its_row_type_passes() {
    const GUIDE: Guide = Guide::new().leaves(&[
        leaf(&["books", "list"]).document(Document::<Vec<Book>>::new(
            "shelf",
            "books-list",
            1,
            "The test rows.",
        )),
        leaf(&["books", "show"]),
        leaf(&["authors", "list"]),
    ]);

    check_guide(&shelf(), &GUIDE, &[]);
}

#[test]
fn a_leaf_column_that_is_not_a_field_of_the_row_type_fails_the_check() {
    const GUIDE: Guide = Guide::new().leaves(&[
        leaf(&["books", "list"]).document(Document::<Vec<MisspeltBook>>::new(
            "shelf",
            "books-list",
            1,
            "The test rows.",
        )),
        leaf(&["books", "show"]),
        leaf(&["authors", "list"]),
    ]);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE, &[])).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         shelf books list: the column `titel` is not a field of the row type, so the table prints `-` in every row of it\n\
         shelf books list: the column `page` is not a field of the row type, so the table prints `-` in every row of it"
    );
}

#[test]
fn a_leaf_column_without_a_description_fails_the_check_naming_the_field() {
    const GUIDE: Guide = Guide::new().leaves(&[
        leaf(&["books", "list"]).document(Document::<Vec<UndocumentedBook>>::new(
            "shelf",
            "books-list",
            1,
            "The test rows.",
        )),
        leaf(&["books", "show"]),
        leaf(&["authors", "list"]),
    ]);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE, &[])).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         shelf books list: the column `title` has no description, because the field `title` of the row type has no doc comment\n\
         shelf books list: the column `pages` has no description, because the field `pages` of the row type has no doc comment"
    );
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "a choices source returns a `Result`"
)]
fn no_choices() -> Result<Vec<crumbkit::pick::Choice>, crumbkit::Error> {
    Ok(Vec::new())
}

#[test]
fn choices_whose_argument_id_names_no_argument_of_the_leaf_fail_the_check() {
    const GUIDE: Guide = Guide::new().leaves(&[
        Leaf::new(&["books", "list"]).examples(RUN),
        Leaf::new(&["books", "show"])
            .examples(&[Example::new(&["2"], "Show a book")])
            .choices(&[
                crumbkit::pick::Choices::new("ids", no_choices),
                crumbkit::pick::Choices::new("id", no_choices),
            ]),
    ]);

    assert_eq!(
        panic_message(|| check_guide(&shelf_with_arguments(), &GUIDE, &[])).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         shelf books show: the choices for `id` name no argument of the command, so a picker never offers them"
    );
}
