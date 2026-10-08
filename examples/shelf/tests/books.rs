//! End-to-end tests of `shelf books list`. Each test runs the built binary and
//! compares its standard output and standard error with golden files under
//! `tests/golden/` (ADR-0004, ADR-0005). `SNAPSHOTS=overwrite cargo test`
//! rewrites the golden files.

use std::path::Path;

use snapbox::cmd::{Command, OutputAssert};
use snapbox::file;

/// Runs the built `shelf` binary with `args`, a cleared environment and XDG
/// directories of its own, and returns its output for assertions.
///
/// `test` names the directory that holds this run's XDG directories.
fn shelf(test: &str, args: &[&str]) -> OutputAssert {
    // RUST-10: each test's XDG directories are paths under cargo's temporary
    // directory for integration tests, and nothing creates or removes them,
    // because shelf reads and writes no XDG directory yet. When shelf writes
    // there, such as for a usage log, replace them with directories that each
    // test creates and removes, such as the temporary directories of snapbox's
    // `dir` feature.
    let xdg = Path::new(env!("CARGO_TARGET_TMPDIR")).join(test);
    Command::new(env!("CARGO_BIN_EXE_shelf"))
        .args(args)
        .env_clear()
        .env("XDG_CONFIG_HOME", xdg.join("config"))
        .env("XDG_DATA_HOME", xdg.join("data"))
        .env("XDG_STATE_HOME", xdg.join("state"))
        .env("XDG_CACHE_HOME", xdg.join("cache"))
        .assert()
}

#[test]
fn books_list_prints_the_compact_columns_as_a_table() {
    shelf("books-list", &["books", "list"])
        .success()
        .stdout_eq(file!["golden/books-list.txt"].raw())
        .stderr_eq("");
}

#[test]
fn books_list_long_adds_the_form_column() {
    shelf("books-list-long", &["books", "list", "--long"])
        .success()
        .stdout_eq(file!["golden/books-list-long.txt"].raw())
        .stderr_eq("");
}

#[test]
fn books_list_json_prints_the_compact_columns_as_one_json_line() {
    shelf("books-list-json", &["books", "list", "--json"])
        .success()
        .stdout_eq(file!["golden/books-list-json.json"].raw())
        .stderr_eq("");
}

#[test]
fn books_list_json_full_prints_every_field_of_every_book() {
    shelf(
        "books-list-json-full",
        &["books", "list", "--json", "--full"],
    )
    .success()
    .stdout_eq(file!["golden/books-list-json-full.json"].raw())
    .stderr_eq("");
}

#[test]
fn books_list_fields_prints_only_the_named_fields_in_order() {
    shelf(
        "books-list-fields",
        &["books", "list", "--fields", "title,year"],
    )
    .success()
    .stdout_eq(file!["golden/books-list-fields.txt"].raw())
    .stderr_eq("");
}

#[test]
fn books_list_json_fields_prints_only_the_named_fields_in_order() {
    shelf(
        "books-list-json-fields",
        &["books", "list", "--json", "--fields", "title,year"],
    )
    .success()
    .stdout_eq(file!["golden/books-list-json-fields.json"].raw())
    .stderr_eq("");
}

#[test]
fn books_list_with_an_unknown_field_exits_with_code_2_and_lists_the_valid_fields() {
    shelf(
        "books-list-unknown-field",
        &["books", "list", "--fields", "title,pages"],
    )
    .code(2)
    .stdout_eq("")
    .stderr_eq(file!["golden/books-list-unknown-field.stderr"].raw());
}
