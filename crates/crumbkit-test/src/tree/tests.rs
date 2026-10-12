//! Tests for the walk over a command tree.

use clap::{Arg, ArgAction, Command};

use super::{walk, walk_visible};

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

/// Returns the path of every command that the walk visits, in order.
fn visited(root: &Command) -> Vec<String> {
    walk(root)
        .iter()
        .map(|visit| visit.path().to_owned())
        .collect()
}

#[test]
fn the_walk_visits_every_command_with_its_full_path_including_hidden_ones() {
    assert_eq!(
        visited(&shelf()),
        [
            "shelf",
            "shelf books",
            "shelf books list",
            "shelf books purge",
            "shelf schema"
        ]
    );
}

#[test]
fn the_walk_sees_the_tree_as_the_tool_built_it_without_the_parts_clap_generates() {
    let shelf = shelf();
    let visits = walk(&shelf);

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
fn a_walk_from_a_subcommand_names_each_path_from_that_subcommand() {
    let shelf = shelf();
    let books = shelf.find_subcommand("books").unwrap();

    assert_eq!(visited(books), ["books", "books list", "books purge"]);
}

#[test]
fn the_visible_walk_leaves_out_each_hidden_command_and_everything_below_it() {
    let shelf = Command::new("shelf")
        .subcommand(
            Command::new("books")
                .subcommand(Command::new("list"))
                .subcommand(Command::new("purge").hide(true)),
        )
        .subcommand(
            Command::new("archive")
                .hide(true)
                .subcommand(Command::new("list")),
        );

    let visits = walk_visible(&shelf);

    let paths: Vec<(&str, &str)> = visits
        .iter()
        .map(|visit| (visit.path(), visit.below()))
        .collect();
    assert_eq!(
        paths,
        [
            ("shelf", ""),
            ("shelf books", "books"),
            ("shelf books list", "books list")
        ]
    );
}
