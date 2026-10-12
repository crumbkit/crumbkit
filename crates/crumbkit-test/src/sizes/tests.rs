//! Tests for the help size check.

use clap::Command;
use crumbkit::help::Guide;

use super::{Budgets, check_help_sizes};
use crate::Budget;
use crate::pages::{Pages, pages};
use crate::testing::panic_message;

/// A tool with one group of two visible leaves and a hidden one.
fn shelf() -> Command {
    Command::new("shelf")
        .about("Browse a small catalogue")
        .subcommand(
            Command::new("books")
                .about("Work with books")
                .subcommand(Command::new("list").about("List the books"))
                .subcommand(Command::new("show").about("Show books"))
                .subcommand(Command::new("purge").hide(true)),
        )
}

/// Returns the size in bytes of the `-h` page of the command at `path`.
///
/// The size depends on the clap features of the build, which the other
/// packages of a workspace build can add, so the tests read it instead of
/// writing it out.
fn size(path: &str) -> usize {
    let shelf = shelf();
    let pages = pages(
        &shelf,
        Pages::rendered(&Guide::new()),
        "-h",
        &mut Vec::new(),
    );
    pages
        .iter()
        .find(|(visit, _)| visit.path() == path)
        .map(|(_, page)| page.len())
        .unwrap()
}

#[test]
fn pages_within_crumbkit_s_default_budgets_pass() {
    check_help_sizes(&shelf(), Pages::rendered(&Guide::new()), &Budgets::new());
}

#[test]
fn a_page_over_its_budget_fails_the_check() {
    const BUDGETS: Budgets = Budgets::new()
        .root(10, "A test.")
        .group(20, "A test.")
        .leaf(30, "A test.");

    assert_eq!(
        panic_message(|| check_help_sizes(&shelf(), Pages::rendered(&Guide::new()), &BUDGETS))
            .unwrap(),
        format!(
            "the command tree breaks crumbkit's help size rules:\n\
             shelf -h: the page has {} bytes, over its budget of 10 bytes\n\
             shelf books -h: the page has {} bytes, over its budget of 20 bytes\n\
             shelf books list -h: the page has {} bytes, over its budget of 30 bytes\n\
             shelf books show -h: the page has {} bytes, over its budget of 30 bytes",
            size("shelf"),
            size("shelf books"),
            size("shelf books list"),
            size("shelf books show"),
        )
    );
}

#[test]
fn a_command_s_budget_wins_over_the_budget_of_its_class() {
    const BUDGETS: Budgets =
        Budgets::new()
            .leaf(1, "A test.")
            .nodes(&[Budget::new("books list", 1_200, "A test.")]);

    assert_eq!(
        panic_message(|| check_help_sizes(&shelf(), Pages::rendered(&Guide::new()), &BUDGETS))
            .unwrap(),
        format!(
            "the command tree breaks crumbkit's help size rules:\n\
             shelf books show -h: the page has {} bytes, over its budget of 1 bytes",
            size("shelf books show"),
        )
    );
}

#[test]
fn a_tool_can_raise_any_budget_above_crumbkit_s_default_with_a_reason() {
    const BUDGETS: Budgets = Budgets::new()
        .root(1_501, "The root lists every resource.")
        .group(801, "Groups name their verbs in full.")
        .leaf(1_201, "Leaves show long examples.")
        .nodes(&[Budget::new(
            "books list",
            2_000,
            "The listing has many filters.",
        )]);

    check_help_sizes(&shelf(), Pages::rendered(&Guide::new()), &BUDGETS);
}

#[test]
fn a_command_s_budget_that_names_no_command_or_a_hidden_one_fails_the_check() {
    const BUDGETS: Budgets = Budgets::new().nodes(&[
        Budget::new("books lst", 600, "A test."),
        Budget::new("books purge", 600, "A test."),
    ]);

    assert_eq!(
        panic_message(|| check_help_sizes(&shelf(), Pages::rendered(&Guide::new()), &BUDGETS))
            .unwrap(),
        "the command tree breaks crumbkit's help size rules:\n\
         the budget of `books lst` names no visible command below the root\n\
         the budget of `books purge` names no visible command below the root"
    );
}

#[test]
fn a_command_whose_page_fails_to_render_reports_only_the_missing_page() {
    const BUDGETS: Budgets = Budgets::new().nodes(&[Budget::new("books list", 600, "A test.")]);
    let shelf = Command::new("shelf")
        .subcommand(Command::new("books").subcommand(Command::new("list").disable_help_flag(true)));

    assert_eq!(
        panic_message(|| check_help_sizes(&shelf, Pages::rendered(&Guide::new()), &BUDGETS))
            .unwrap(),
        "the command tree breaks crumbkit's help size rules:\n\
         shelf books list -h: the command line prints no help page"
    );
}

#[cfg(unix)]
#[test]
fn a_binary_s_page_is_measured_from_what_it_prints_on_standard_output() {
    const BUDGETS: Budgets = Budgets::new().leaf(30, "A test.");
    let dir = crate::temp::TempDir::new("sizes-binary").unwrap();
    let list = "a".repeat(31);
    let binary = crate::testing::help_binary(
        &dir,
        &[
            ("-h", "shelf\n"),
            ("books -h", "books\n"),
            ("books list -h", &list),
            ("books show -h", "show\n"),
        ],
    );
    let examples = crate::Examples::new(&binary, "");
    let guide = Guide::new();

    assert_eq!(
        panic_message(|| check_help_sizes(&shelf(), Pages::binary(&guide, &examples), &BUDGETS))
            .unwrap(),
        "the command tree breaks crumbkit's help size rules:\n\
         shelf books list -h: the page has 31 bytes, over its budget of 30 bytes"
    );
}
