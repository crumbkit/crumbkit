//! End-to-end tests of `shelf books remove`, which change shelf's state.
//! Each test runs the built binary more than once in one temporary
//! directory, so a later run sees what an earlier run removed. The golden
//! files are under `tests/golden/`, and `SNAPSHOTS=overwrite cargo test`
//! rewrites them.

use std::path::Path;

use clap::CommandFactory as _;
use crumbkit::codes;
use crumbkit_test::{Examples, Failing, check_twin};
use snapbox::cmd::{Command, OutputAssert};
use snapbox::file;

/// Runs the built `shelf` binary with `args`, a cleared environment and XDG
/// directories under `dir`, and returns its output for assertions. Standard
/// input is empty, so it is never a terminal.
fn run(dir: &Path, args: &[&str]) -> OutputAssert {
    Command::new(env!("CARGO_BIN_EXE_shelf"))
        .args(args)
        .current_dir(dir)
        .env_clear()
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_DATA_HOME", dir.join("data"))
        .env("XDG_STATE_HOME", dir.join("state"))
        .env("XDG_CACHE_HOME", dir.join("cache"))
        .assert()
}

#[test]
fn books_remove_dry_run_prints_the_book_and_the_dry_run_line_on_standard_error_and_list_still_holds_it()
 {
    let dir = tempfile::tempdir().unwrap();

    run(dir.path(), &["books", "remove", "2", "--dry-run"])
        .success()
        .stdout_eq(file!["golden/books-remove-2-dry-run.txt"].raw())
        .stderr_eq(file!["golden/books-remove-2-dry-run.stderr"].raw());

    run(dir.path(), &["books", "list"])
        .success()
        .stdout_eq(file!["golden/examples/shelf-books-list.txt"].raw());
}

#[test]
fn books_remove_dry_run_json_marks_the_document_with_dry_run_true() {
    let dir = tempfile::tempdir().unwrap();

    run(dir.path(), &["books", "remove", "2", "--dry-run", "--json"])
        .success()
        .stdout_eq(file!["golden/books-remove-2-dry-run-json.json"].raw())
        .stderr_eq("");
}

#[test]
fn books_remove_without_yes_or_a_terminal_fails_with_needs_yes_and_removes_nothing() {
    let dir = tempfile::tempdir().unwrap();

    run(dir.path(), &["books", "remove", "2"])
        .code(1)
        .stdout_eq("")
        .stderr_eq(file!["golden/books-remove-2.stderr"].raw());
    run(dir.path(), &["books", "remove", "2", "--json"])
        .code(1)
        .stdout_eq(file!["golden/books-remove-2-json.json"].raw());

    // The twins run in directories of their own, so the runs above are the
    // ones that must leave book 2 in this directory (TEST-1).
    let shelf = Examples::new(env!("CARGO_BIN_EXE_shelf"), "");
    for refused in [
        &["books", "remove", "2"][..],
        &["books", "remove", "2", "--json"],
    ] {
        let mut admitted = refused.to_vec();
        admitted.insert(3, "--yes");
        check_twin(
            &shelf::Cli::command(),
            &shelf::GUIDE,
            &shelf,
            &Failing::new(&codes::CONFIRM_NEEDS_YES, refused),
            &admitted,
        );
    }

    run(dir.path(), &["books", "list"])
        .success()
        .stdout_eq(file!["golden/examples/shelf-books-list.txt"].raw());
}

#[test]
fn books_remove_yes_removes_the_book_so_list_skips_it_and_show_cannot_find_it() {
    let dir = tempfile::tempdir().unwrap();

    run(dir.path(), &["books", "remove", "2", "--yes"])
        .success()
        .stdout_eq(file!["golden/examples/shelf-books-remove-2-yes.txt"].raw())
        .stderr_eq("");

    run(dir.path(), &["books", "list"])
        .success()
        .stdout_eq(file!["golden/books-list-after-remove-2.txt"].raw());
    run(dir.path(), &["books", "show", "2"])
        .code(1)
        .stderr_eq(file!["golden/books-show-2-after-remove-2.stderr"].raw());
}

#[test]
fn books_remove_yes_a_second_time_succeeds_and_prints_the_same_book() {
    let dir = tempfile::tempdir().unwrap();
    run(dir.path(), &["books", "remove", "2", "--yes"]).success();

    run(dir.path(), &["books", "remove", "2", "--yes"])
        .success()
        .stdout_eq(file!["golden/examples/shelf-books-remove-2-yes.txt"].raw())
        .stderr_eq("");
}

#[test]
fn books_list_after_every_book_is_removed_prints_the_header_and_the_empty_note_on_standard_error() {
    let dir = tempfile::tempdir().unwrap();
    run(
        dir.path(),
        &["books", "remove", "1", "2", "3", "4", "5", "6", "--yes"],
    )
    .success();

    run(dir.path(), &["books", "list"])
        .success()
        .stdout_eq(file!["golden/books-list-after-remove-all.txt"].raw())
        .stderr_eq(file!["golden/books-list-after-remove-all.stderr"].raw());
    run(dir.path(), &["books", "list", "--json"])
        .success()
        .stdout_eq(file!["golden/books-list-json-after-remove-all.json"].raw())
        .stderr_eq("");
}

#[test]
fn books_show_after_remove_notes_the_removed_books_before_their_failures_and_json_leaves_the_note_out()
 {
    let dir = tempfile::tempdir().unwrap();
    run(dir.path(), &["books", "remove", "2", "5", "--yes"]).success();

    run(dir.path(), &["books", "show", "1", "2", "5"])
        .code(1)
        .stdout_eq(file!["golden/books-show-1-2-5-after-remove-2-5.txt"].raw())
        .stderr_eq(file!["golden/books-show-1-2-5-after-remove-2-5.stderr"].raw());
    run(dir.path(), &["books", "show", "1", "2", "5", "--json"])
        .code(1)
        .stdout_eq(file!["golden/books-show-1-2-5-after-remove-2-5-json.json"].raw())
        .stderr_eq("");
}

#[test]
fn books_summarize_after_remove_notes_the_removed_books_it_leaves_out_and_json_leaves_the_note_out()
{
    let dir = tempfile::tempdir().unwrap();
    run(dir.path(), &["books", "remove", "2", "5", "--yes"]).success();

    run(dir.path(), &["books", "summarize"])
        .success()
        .stdout_eq(file!["golden/books-summarize-after-remove-2-5.txt"].raw())
        .stderr_eq(file!["golden/books-summarize-after-remove-2-5.stderr"].raw());
    run(dir.path(), &["books", "summarize", "--json"])
        .success()
        .stdout_eq(file!["golden/books-summarize-after-remove-2-5-json.json"].raw())
        .stderr_eq("");
}
