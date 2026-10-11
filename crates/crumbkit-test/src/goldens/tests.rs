//! Tests for the help golden check.

use std::fs;

use clap::Command;
use crumbkit::help::Guide;
use snapbox::Assert;
use snapbox::assert::Action;

use super::check;
use crate::temp::TempDir;
use crate::testing::panic_message;

/// A tool with one group and one leaf.
fn shelf() -> Command {
    Command::new("shelf")
        .about("Browse a small catalogue")
        .subcommand(
            Command::new("books")
                .about("Work with books")
                .subcommand(Command::new("list").about("List the books")),
        )
}

/// The golden files of `shelf()`, sorted by name.
const FILES: [&str; 6] = [
    "shelf-books-h.txt",
    "shelf-books-help.txt",
    "shelf-books-list-h.txt",
    "shelf-books-list-help.txt",
    "shelf-h.txt",
    "shelf-help.txt",
];

/// Runs the check on `shelf()` against `dir` with `action`.
fn run(dir: &TempDir, action: Action) {
    check(
        &shelf(),
        &Guide::new(),
        dir.path(),
        &Assert::new().action(action),
    );
}

#[test]
fn overwrite_writes_a_golden_file_for_each_page_and_deletes_a_stale_one() {
    let dir = TempDir::new("goldens-overwrite").unwrap();
    dir.write("shelf-authors-h.txt", "Work with authors\n");
    dir.write("notes.md", "Only .txt files are golden files.\n");

    run(&dir, Action::Overwrite);

    let mut expected: Vec<&str> = FILES.to_vec();
    expected.insert(0, "notes.md");
    assert_eq!(dir.names(), expected);
    let page = fs::read_to_string(dir.path().join("shelf-books-list-h.txt")).unwrap();
    assert!(page.starts_with("List the books\n\nUsage:"), "{page}");
}

#[test]
fn golden_files_that_hold_every_page_pass() {
    let dir = TempDir::new("goldens-pass").unwrap();
    run(&dir, Action::Overwrite);

    run(&dir, Action::Verify);
}

#[test]
fn a_page_without_a_golden_file_fails_the_check() {
    let dir = TempDir::new("goldens-missing").unwrap();
    let file = |name: &str| dir.path().join(name).display().to_string();

    assert_eq!(
        panic_message(|| run(&dir, Action::Verify)).unwrap(),
        format!(
            "the command tree breaks crumbkit's golden help rules:\n\
             shelf -h: the golden file `{}` is missing, and `SNAPSHOTS=overwrite` writes it\n\
             shelf books -h: the golden file `{}` is missing, and `SNAPSHOTS=overwrite` writes it\n\
             shelf books list -h: the golden file `{}` is missing, and `SNAPSHOTS=overwrite` writes it\n\
             shelf --help: the golden file `{}` is missing, and `SNAPSHOTS=overwrite` writes it\n\
             shelf books --help: the golden file `{}` is missing, and `SNAPSHOTS=overwrite` writes it\n\
             shelf books list --help: the golden file `{}` is missing, and `SNAPSHOTS=overwrite` writes it",
            file("shelf-h.txt"),
            file("shelf-books-h.txt"),
            file("shelf-books-list-h.txt"),
            file("shelf-help.txt"),
            file("shelf-books-help.txt"),
            file("shelf-books-list-help.txt"),
        )
    );
}

#[test]
fn a_golden_file_whose_content_differs_fails_the_check() {
    let dir = TempDir::new("goldens-differs").unwrap();
    run(&dir, Action::Overwrite);
    dir.write("shelf-books-h.txt", "Work with the books\n");

    let message = panic_message(|| run(&dir, Action::Verify)).unwrap();

    let heading = format!(
        "the command tree breaks crumbkit's golden help rules:\n\
         shelf books -h: the page differs from the golden file `{}`:\n",
        dir.path().join("shelf-books-h.txt").display()
    );
    assert!(message.starts_with(&heading), "{message}");
    assert!(message.contains("Work with the books"), "{message}");
    assert_eq!(
        message.matches(": the page differs").count(),
        1,
        "{message}"
    );
}

#[test]
fn a_stale_golden_file_fails_the_check() {
    let dir = TempDir::new("goldens-stale").unwrap();
    run(&dir, Action::Overwrite);
    dir.write("shelf-authors-h.txt", "Work with authors\n");

    assert_eq!(
        panic_message(|| run(&dir, Action::Verify)).unwrap(),
        format!(
            "the command tree breaks crumbkit's golden help rules:\n\
             the golden file `{}` belongs to no help page, and `SNAPSHOTS=overwrite` deletes it",
            dir.path().join("shelf-authors-h.txt").display()
        )
    );
}

#[test]
fn two_pages_with_one_golden_file_name_fail_the_check_naming_both_command_lines() {
    let twins = shelf().subcommand(Command::new("books-list").about("List the books again"));
    let dir = TempDir::new("goldens-collision").unwrap();

    assert_eq!(
        panic_message(|| check(
            &twins,
            &Guide::new(),
            dir.path(),
            &Assert::new().action(Action::Overwrite)
        ))
        .unwrap(),
        "the command tree breaks crumbkit's golden help rules:\n\
         shelf books-list -h: the page's golden file name `shelf-books-list-h.txt` is also the name for the page `shelf books list -h`\n\
         shelf books-list --help: the page's golden file name `shelf-books-list-help.txt` is also the name for the page `shelf books list --help`"
    );
}
