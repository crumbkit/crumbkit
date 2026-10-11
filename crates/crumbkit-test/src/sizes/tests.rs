//! Tests for the help size check.

use clap::Command;
use crumbkit::help::Guide;

use super::{Budgets, check_help_sizes};
use crate::pages::pages;
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
    let pages = pages(&shelf, &Guide::new(), "-h", &mut Vec::new());
    pages
        .iter()
        .find(|(visit, _)| visit.path() == path)
        .map(|(_, page)| page.len())
        .unwrap()
}

#[test]
fn pages_within_crumbkit_s_default_budgets_pass() {
    check_help_sizes(&shelf(), &Guide::new(), &Budgets::new());
}

#[test]
fn a_page_over_its_budget_fails_the_check() {
    const BUDGETS: Budgets = Budgets::new().root(10).group(20).leaf(30);

    assert_eq!(
        panic_message(|| check_help_sizes(&shelf(), &Guide::new(), &BUDGETS)).unwrap(),
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
    const BUDGETS: Budgets = Budgets::new().leaf(1).nodes(&[("books list", 1_200)]);

    assert_eq!(
        panic_message(|| check_help_sizes(&shelf(), &Guide::new(), &BUDGETS)).unwrap(),
        format!(
            "the command tree breaks crumbkit's help size rules:\n\
             shelf books show -h: the page has {} bytes, over its budget of 1 bytes",
            size("shelf books show"),
        )
    );
}

#[test]
fn a_class_budget_above_crumbkit_s_default_fails_the_check() {
    const BUDGETS: Budgets = Budgets::new().root(1_501).group(801).leaf(1_201);

    assert_eq!(
        panic_message(|| check_help_sizes(&shelf(), &Guide::new(), &BUDGETS)).unwrap(),
        "the command tree breaks crumbkit's help size rules:\n\
         the root budget of 1501 bytes is above crumbkit's default of 1500 bytes, and a tool can lower a budget and cannot raise one\n\
         the group budget of 801 bytes is above crumbkit's default of 800 bytes, and a tool can lower a budget and cannot raise one\n\
         the leaf budget of 1201 bytes is above crumbkit's default of 1200 bytes, and a tool can lower a budget and cannot raise one"
    );
}

#[test]
fn a_command_s_budget_above_crumbkit_s_default_for_its_class_fails_the_check() {
    const BUDGETS: Budgets = Budgets::new().nodes(&[("books", 801), ("books list", 1_201)]);

    assert_eq!(
        panic_message(|| check_help_sizes(&shelf(), &Guide::new(), &BUDGETS)).unwrap(),
        "the command tree breaks crumbkit's help size rules:\n\
         shelf books: the budget of 801 bytes is above crumbkit's default of 800 bytes for a group, and a tool can lower a budget and cannot raise one\n\
         shelf books list: the budget of 1201 bytes is above crumbkit's default of 1200 bytes for a leaf, and a tool can lower a budget and cannot raise one"
    );
}

#[test]
fn a_command_s_budget_that_names_no_command_or_a_hidden_one_fails_the_check() {
    const BUDGETS: Budgets = Budgets::new().nodes(&[("books lst", 600), ("books purge", 600)]);

    assert_eq!(
        panic_message(|| check_help_sizes(&shelf(), &Guide::new(), &BUDGETS)).unwrap(),
        "the command tree breaks crumbkit's help size rules:\n\
         the budget of `books lst` names no visible command below the root\n\
         the budget of `books purge` names no visible command below the root"
    );
}

#[test]
fn a_command_whose_page_fails_to_render_reports_only_the_missing_page() {
    const BUDGETS: Budgets = Budgets::new().nodes(&[("books list", 600)]);
    let shelf = Command::new("shelf")
        .subcommand(Command::new("books").subcommand(Command::new("list").disable_help_flag(true)));

    assert_eq!(
        panic_message(|| check_help_sizes(&shelf, &Guide::new(), &BUDGETS)).unwrap(),
        "the command tree breaks crumbkit's help size rules:\n\
         shelf books list -h: the command line prints no help page"
    );
}
