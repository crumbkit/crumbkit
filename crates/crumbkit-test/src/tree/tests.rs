//! Tests for the walk over a command tree.

use clap::{Arg, ArgAction, Command};

use super::walk;

/// A tool with one resource that has a visible and a hidden verb.
fn shelf() -> Command {
    Command::new("shelf")
        .version("1.0.0")
        .arg(
            Arg::new("json")
                .long("json")
                .global(true)
                .action(ArgAction::SetTrue),
        )
        .subcommand(
            Command::new("books")
                .subcommand(Command::new("list"))
                .subcommand(Command::new("purge").hide(true)),
        )
        .subcommand(Command::new("schema"))
}

/// Returns the path of every command that the walk visits, in order, and the
/// problems it finds.
fn visited(root: &Command, exempt: &[&str]) -> (Vec<String>, Vec<String>) {
    let mut problems = Vec::new();
    let paths = walk(root, exempt, &mut problems)
        .iter()
        .map(|visit| visit.path().to_owned())
        .collect();
    (paths, problems)
}

#[test]
fn the_walk_visits_every_command_with_its_full_path_including_hidden_ones() {
    let (paths, problems) = visited(&shelf(), &[]);

    assert_eq!(
        paths,
        [
            "shelf",
            "shelf books",
            "shelf books list",
            "shelf books purge",
            "shelf schema"
        ]
    );
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn the_walk_sees_the_tree_as_the_tool_built_it_without_the_parts_clap_generates() {
    let shelf = shelf();
    let visits = walk(&shelf, &[], &mut Vec::new());

    let arguments: Vec<(&str, &str)> = visits
        .iter()
        .flat_map(|visit| {
            visit
                .command()
                .get_arguments()
                .map(|arg| (visit.path(), arg.get_id().as_str()))
        })
        .collect();
    assert_eq!(arguments, [("shelf", "json")]);
    assert!(
        visits
            .iter()
            .all(|visit| visit.command().get_name() != "help")
    );
}

#[test]
fn an_exempt_command_and_everything_below_it_are_skipped() {
    let (paths, problems) = visited(&shelf(), &["books", "schema"]);

    assert_eq!(paths, ["shelf"]);
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn an_exemption_names_a_command_by_its_path_below_the_command_passed_in() {
    let (paths, _) = visited(&shelf(), &["books purge"]);

    assert_eq!(
        paths,
        ["shelf", "shelf books", "shelf books list", "shelf schema"]
    );
}

#[test]
fn an_exemption_that_names_no_command_is_a_problem() {
    let (_, problems) = visited(&shelf(), &["books remove", "shelf", "purge"]);

    assert_eq!(
        problems,
        [
            "the exemption `books remove` names no command",
            "the exemption `shelf` names no command",
            "the exemption `purge` names no command",
        ]
    );
}

#[test]
fn a_walk_from_a_subcommand_names_each_path_from_that_subcommand() {
    let shelf = shelf();
    let books = shelf.find_subcommand("books").unwrap();

    let (paths, problems) = visited(books, &["purge"]);

    assert_eq!(paths, ["books", "books list"]);
    assert!(problems.is_empty(), "{problems:?}");
}
