//! Tests for the path weight check.

use clap::Command;
use crumbkit::help::{Example, Guide, Leaf};

use super::{PathBudgets, check_path_weights, output_weight, path_totals};
use crate::pages::pages;
use crate::temp::TempDir;
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

/// A guide whose `books list` has two examples.
const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "list"]).examples(&[
    Example::new(&[], "Print the first page of books"),
    Example::new(&["--all"], "Print every book"),
])]);

/// Returns the size in bytes of the `-h` page of the command at `path`.
///
/// The size depends on the clap features of the build, so the tests read it
/// instead of writing it out.
fn size(path: &str) -> u64 {
    let shelf = shelf();
    let pages = pages(&shelf, &GUIDE, "-h", &mut Vec::new());
    pages
        .iter()
        .find(|(visit, _)| visit.path() == path)
        .map(|(_, page)| page.len() as u64)
        .unwrap()
}

/// Returns a golden directory that holds the output of both examples of
/// `books list`, the larger of 40 bytes.
fn goldens() -> TempDir {
    let dir = TempDir::new("path-weights").unwrap();
    dir.write("shelf-books-list.txt", &"a".repeat(25));
    dir.write("shelf-books-list-all.txt", &"a".repeat(40));
    dir
}

#[test]
fn paths_within_crumbkit_s_default_budget_pass() {
    let dir = goldens();

    check_path_weights(
        &shelf(),
        &GUIDE,
        &PathBudgets::new(dir.path().to_str().unwrap()),
    );
}

#[test]
fn a_path_over_its_budget_fails_with_its_pages_and_output() {
    let dir = goldens();
    let pages = size("shelf") + size("shelf books") + size("shelf books list");
    let budget = pages + 39;
    let leaves = [("books list", budget)];
    let budgets = PathBudgets::new(dir.path().to_str().unwrap()).nodes(&leaves);

    assert_eq!(
        panic_message(|| check_path_weights(&shelf(), &GUIDE, &budgets)).unwrap(),
        format!(
            "the command tree breaks crumbkit's path weight rules:\n\
             shelf books list: the path reads {} bytes, {pages} of -h pages and 40 of output, over its budget of {budget} bytes",
            pages + 40,
        )
    );
}

#[test]
fn a_command_s_budget_wins_over_the_path_budget() {
    let dir = goldens();
    let budget = size("shelf") + size("shelf books") + size("shelf books list") + 40;
    let leaves = [("books list", budget)];
    let budgets = PathBudgets::new(dir.path().to_str().unwrap())
        .path(size("shelf") + size("shelf books") + size("shelf books show"))
        .nodes(&leaves);

    check_path_weights(&shelf(), &GUIDE, &budgets);
}

#[test]
fn a_budget_above_crumbkit_s_default_fails_the_check() {
    let dir = goldens();
    let budgets = PathBudgets::new(dir.path().to_str().unwrap())
        .path(8_001)
        .nodes(&[
            ("books list", 8_001),
            ("books purge", 100),
            ("books gone", 100),
        ]);

    assert_eq!(
        panic_message(|| check_path_weights(&shelf(), &GUIDE, &budgets)).unwrap(),
        "the command tree breaks crumbkit's path weight rules:\n\
         the path budget of 8001 bytes is above crumbkit's default of 8000 bytes, and a tool can lower a budget and cannot raise one\n\
         shelf books list: the budget of 8001 bytes is above crumbkit's default of 8000 bytes, and a tool can lower a budget and cannot raise one\n\
         the budget of `books purge` names no visible command below the root\n\
         the budget of `books gone` names no visible command below the root"
    );
}

#[test]
fn a_golden_directory_that_is_a_file_fails_the_check() {
    let dir = goldens();
    let file = dir.path().join("shelf-books-list.txt");
    let budgets = PathBudgets::new(file.to_str().unwrap());

    let message = panic_message(|| check_path_weights(&shelf(), &GUIDE, &budgets)).unwrap();

    assert!(
        message.starts_with(&format!(
            "the command tree breaks crumbkit's path weight rules:\n\
             shelf books list: the golden file {} cannot be read: ",
            file.join("shelf-books-list.txt").display()
        )),
        "{message}"
    );
}

#[test]
fn a_path_total_adds_every_short_page_from_the_root_down_and_the_command_s_output() {
    let rows = [
        (0, 500, None),
        (1, 300, None),
        (2, 200, Some(150)),
        (2, 250, Some(90)),
        (1, 100, None),
        (2, 400, Some(2_000)),
    ];

    assert_eq!(path_totals(&rows), [500, 800, 1_150, 1_140, 600, 3_000]);
}

#[test]
fn a_command_without_output_weight_totals_its_short_pages_alone() {
    assert_eq!(path_totals(&[(0, 500, None), (1, 80, None)]), [500, 580]);
}

#[test]
fn a_root_without_subcommands_totals_its_own_page_and_output() {
    assert_eq!(path_totals(&[(0, 700, Some(12))]), [712]);
}

/// The command lines of the two examples of `shelf books list`.
fn examples() -> Vec<String> {
    vec![
        "shelf books list".to_owned(),
        "shelf books list --all".to_owned(),
    ]
}

#[test]
fn the_output_weight_is_the_size_of_the_largest_golden_file_of_the_examples() {
    let dir = goldens();
    dir.write("shelf-books-show-2-5.txt", &"a".repeat(100));

    assert_eq!(output_weight(dir.path(), &examples()).unwrap(), Some(40));
}

#[test]
fn examples_without_golden_files_have_no_output_weight() {
    let dir = TempDir::new("no-goldens").unwrap();
    dir.write("shelf-books-list-all.txt", "every book\n");

    assert_eq!(output_weight(dir.path(), &examples()).unwrap(), Some(11));
    assert_eq!(
        output_weight(&dir.path().join("missing"), &examples()).unwrap(),
        None
    );
    assert_eq!(output_weight(dir.path(), &[]).unwrap(), None);
}

#[test]
fn a_golden_directory_that_is_a_file_has_an_unreadable_golden_file() {
    let dir = goldens();
    let file = dir.path().join("shelf-books-list.txt");

    let error = output_weight(&file, &examples()).unwrap_err();

    assert_eq!(error.path(), file.join("shelf-books-list.txt"));
}
