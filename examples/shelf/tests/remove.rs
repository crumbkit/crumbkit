//! End-to-end tests of `shelf books remove`, which change shelf's state.
//! Each test runs the built binary more than once in one temporary
//! directory, so a later run sees what an earlier run removed. The golden
//! files are under `tests/golden/`, and `SNAPSHOTS=overwrite cargo test`
//! rewrites them.

use std::path::Path;

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
fn books_remove_dry_run_prints_the_book_and_the_dry_run_line_and_list_still_holds_it() {
    let dir = tempfile::tempdir().unwrap();

    run(dir.path(), &["books", "remove", "2", "--dry-run"])
        .success()
        .stdout_eq(file!["golden/books-remove-2-dry-run.txt"].raw())
        .stderr_eq("");

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
