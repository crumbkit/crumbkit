//! Tests for the help pages that the help checks read.

use clap::{Arg, ArgAction, Command};
use crumbkit::help::{Example, Goal, Group, Guide, Leaf};

use super::pages;

/// A tool with a global flag, a guided root and one leaf with examples.
fn tool() -> Command {
    Command::new("tool")
        .arg(
            Arg::new("json")
                .long("json")
                .global(true)
                .action(ArgAction::SetTrue)
                .help("Print JSON"),
        )
        .subcommand(Command::new("books").subcommand(
            Command::new("list").arg(Arg::new("all").long("all").action(ArgAction::SetTrue)),
        ))
}

/// The guide of `tool()`.
const GUIDE: Guide = Guide::new()
    .groups(&[Group::new(&[], &[Goal::new("Read", &["books"])])])
    .leaves(&[
        Leaf::new(&["books", "list"]).examples(&[Example::new(&["--all"], "Print every book")])
    ]);

#[test]
fn each_visible_command_has_the_page_that_crumbkit_prints_for_the_flag_with_the_guide() {
    let tool = tool();
    let mut problems = Vec::new();

    let short = pages(&tool, &GUIDE, "-h", &mut problems);
    let long = pages(&tool, &GUIDE, "--help", &mut problems);

    let paths: Vec<&str> = short.iter().map(|(visit, _)| visit.path()).collect();
    assert_eq!(paths, ["tool", "tool books", "tool books list"]);
    assert!(short[0].1.contains("\nRead\n  books\n"), "{}", short[0].1);
    assert!(
        short[2].1.ends_with(
            "\nExamples:\n  tool books list --all\n\nGlobal flags: --json. `tool books list --help` describes them.\n"
        ),
        "{}",
        short[2].1
    );
    assert!(
        long[2]
            .1
            .ends_with("\nExamples:\n  tool books list --all\n          Print every book\n"),
        "{}",
        long[2].1
    );
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn a_command_line_that_prints_no_help_page_is_a_problem() {
    let tool = Command::new("tool").subcommand(Command::new("list").disable_help_flag(true));
    let mut problems = Vec::new();

    let pages = pages(&tool, &Guide::new(), "-h", &mut problems);

    let paths: Vec<&str> = pages.iter().map(|(visit, _)| visit.path()).collect();
    assert_eq!(paths, ["tool"]);
    assert_eq!(
        problems,
        ["tool list -h: the command line prints no help page"]
    );
}
