//! Tests for the help pages that the help checks read.

use clap::{Arg, ArgAction, Command};
use crumbkit::help::{Example, Goal, Group, Guide, Leaf};

use super::{Pages, pages};
use crate::examples::Examples;
use crate::temp::TempDir;
use crate::testing::help_binary;

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

    let short = pages(&tool, Pages::rendered(&GUIDE), "-h", &mut problems);
    let long = pages(&tool, Pages::rendered(&GUIDE), "--help", &mut problems);

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

    let pages = pages(&tool, Pages::rendered(&Guide::new()), "-h", &mut problems);

    let paths: Vec<&str> = pages.iter().map(|(visit, _)| visit.path()).collect();
    assert_eq!(paths, ["tool"]);
    assert_eq!(
        problems,
        ["tool list -h: the command line prints no help page"]
    );
}

/// Fails the test, for a guide whose personal aliases no page may read.
fn unread(_: &Command) -> Vec<String> {
    panic!("a help page read the personal aliases");
}

#[test]
fn the_pages_leave_out_the_personal_aliases_so_they_match_on_every_machine() {
    let tool = tool();
    let mut problems = Vec::new();

    let guided = pages(
        &tool,
        Pages::rendered(&GUIDE.personal_aliases(unread)),
        "-h",
        &mut problems,
    );
    let plain = pages(&tool, Pages::rendered(&GUIDE), "-h", &mut problems);

    assert_eq!(guided[0].1, plain[0].1);
    assert!(problems.is_empty(), "{problems:?}");
}

#[cfg(unix)]
#[test]
fn binary_pages_run_with_the_examples_environment_working_directory_and_fresh_xdg_directories() {
    let dir = TempDir::new("pages-binary").unwrap();
    dir.write_script(
        "tool",
        "#!/bin/sh\nprintf '%s|%s|%s|%s' \"$*\" \"$TOOL_NAME\" \"$(pwd -P)\" \"$XDG_CONFIG_HOME\"\n",
    );
    let work = TempDir::new("pages-binary-work").unwrap();
    let work_path = work.path().display().to_string();
    let binary = dir.path().join("tool").display().to_string();
    let examples = Examples::new(&binary, "")
        .env(&[("TOOL_NAME", "tool")])
        .current_dir(&work_path);
    let tool = tool();
    let mut problems = Vec::new();

    let pages = pages(&tool, Pages::binary(&GUIDE, &examples), "-h", &mut problems);

    let real = std::fs::canonicalize(work.path())
        .unwrap()
        .display()
        .to_string();
    let fields: Vec<Vec<&str>> = pages
        .iter()
        .map(|(_, page)| page.split('|').collect())
        .collect();
    let words: Vec<&str> = fields.iter().map(|page| page[0]).collect();
    assert_eq!(words, ["-h", "books -h", "books list -h"]);
    assert!(
        fields
            .iter()
            .all(|page| page[1] == "tool" && page[2] == real),
        "{fields:?}"
    );
    assert!(
        fields.iter().all(|page| page[3].ends_with("/config")),
        "{fields:?}"
    );
    assert_ne!(fields[0][3], fields[1][3]);
    assert!(problems.is_empty(), "{problems:?}");
}

#[cfg(unix)]
#[test]
fn a_binary_page_that_exits_with_a_failure_is_a_command_line_that_prints_no_help_page() {
    let dir = TempDir::new("pages-binary-fails").unwrap();
    let binary = help_binary(
        &dir,
        &[("-h", "Usage: tool\n"), ("books -h", "Usage: tool books\n")],
    );
    let examples = Examples::new(&binary, "");
    let tool = tool();
    let mut problems = Vec::new();

    let pages = pages(&tool, Pages::binary(&GUIDE, &examples), "-h", &mut problems);

    let paths: Vec<&str> = pages.iter().map(|(visit, _)| visit.path()).collect();
    assert_eq!(paths, ["tool", "tool books"]);
    assert_eq!(pages[1].1, "Usage: tool books\n");
    assert_eq!(
        problems,
        [
            "tool books list -h: the command line prints no help page, and the binary ends with exit status: 2"
        ]
    );
}
