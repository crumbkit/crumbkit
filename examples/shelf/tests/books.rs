//! End-to-end tests of `shelf books list` and `shelf books show`. Each test
//! runs the built binary and compares its standard output and standard error
//! with golden files under `tests/golden/` (ADR-0004, ADR-0005).
//! `SNAPSHOTS=overwrite cargo test` rewrites the golden files.

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
fn books_list_prints_the_first_4_books_a_footer_naming_the_rest_and_the_next_line() {
    shelf("books-list", &["books", "list"])
        .success()
        .stdout_eq(file!["golden/books-list.txt"].raw())
        .stderr_eq("");
}

#[test]
fn books_list_long_adds_the_form_column_and_keeps_the_footer() {
    shelf("books-list-long", &["books", "list", "--long"])
        .success()
        .stdout_eq(file!["golden/books-list-long.txt"].raw())
        .stderr_eq("");
}

#[test]
fn books_list_json_prints_the_compact_columns_and_the_total_as_one_json_line() {
    shelf("books-list-json", &["books", "list", "--json"])
        .success()
        .stdout_eq(file!["golden/books-list-json.json"].raw())
        .stderr_eq("");
}

#[test]
fn books_list_json_full_prints_every_field_of_the_first_4_books_and_the_total() {
    shelf(
        "books-list-json-full",
        &["books", "list", "--json", "--full"],
    )
    .success()
    .stdout_eq(file!["golden/books-list-json-full.json"].raw())
    .stderr_eq("");
}

#[test]
fn books_list_fields_prints_only_the_named_fields_in_order_and_the_footer() {
    shelf(
        "books-list-fields",
        &["books", "list", "--fields", "title,year"],
    )
    .success()
    .stdout_eq(file!["golden/books-list-fields.txt"].raw())
    .stderr_eq("");
}

#[test]
fn books_list_json_fields_prints_only_the_named_fields_in_order_and_the_total() {
    shelf(
        "books-list-json-fields",
        &["books", "list", "--json", "--fields", "title,year"],
    )
    .success()
    .stdout_eq(file!["golden/books-list-json-fields.json"].raw())
    .stderr_eq("");
}

#[test]
fn books_list_with_an_unknown_field_exits_with_code_2_and_hints_at_the_valid_fields() {
    shelf(
        "books-list-unknown-field",
        &["books", "list", "--fields", "title,pages"],
    )
    .code(2)
    .stdout_eq("")
    .stderr_eq(file!["golden/books-list-unknown-field.stderr"].raw());
}

#[test]
fn books_list_all_prints_every_book_and_the_next_line_without_a_footer() {
    shelf("books-list-all", &["books", "list", "--all"])
        .success()
        .stdout_eq(file!["golden/books-list-all.txt"].raw())
        .stderr_eq("");
}

#[test]
fn books_list_limit_2_prints_the_first_2_books_and_the_footer() {
    shelf("books-list-limit-2", &["books", "list", "--limit", "2"])
        .success()
        .stdout_eq(file!["golden/books-list-limit-2.txt"].raw())
        .stderr_eq("");
}

#[test]
fn books_list_limit_0_is_refused_with_code_2() {
    shelf("books-list-limit-0", &["books", "list", "--limit", "0"])
        .code(2)
        .stdout_eq("")
        .stderr_eq(file!["golden/books-list-limit-0.stderr"].raw());
}

#[test]
fn books_list_all_with_limit_is_refused_with_code_2() {
    shelf(
        "books-list-all-limit-2",
        &["books", "list", "--all", "--limit", "2"],
    )
    .code(2)
    .stdout_eq("")
    .stderr_eq(file!["golden/books-list-all-limit-2.stderr"].raw());
}

#[test]
fn books_list_json_all_prints_every_book_and_the_total() {
    shelf("books-list-json-all", &["books", "list", "--json", "--all"])
        .success()
        .stdout_eq(file!["golden/books-list-json-all.json"].raw())
        .stderr_eq("");
}

#[test]
fn books_list_short_help_shows_the_paging_flags_and_names_the_output_flags_on_one_line() {
    shelf("books-list-h", &["books", "list", "-h"])
        .success()
        .stdout_eq(file!["golden/books-list-h.txt"].raw())
        .stderr_eq("");
}

#[test]
fn books_show_with_comma_separated_ids_is_refused_with_code_2() {
    shelf("books-show-2-comma-5", &["books", "show", "2,5"])
        .code(2)
        .stdout_eq("")
        .stderr_eq(file!["golden/books-show-2-comma-5.stderr"].raw());
}

#[test]
fn books_show_prints_the_books_in_the_order_given() {
    shelf("books-show-5-2", &["books", "show", "5", "2"])
        .success()
        .stdout_eq(file!["golden/books-show-5-2.txt"].raw())
        .stderr_eq("");
}

#[test]
fn books_show_with_only_an_id_outside_the_catalogue_prints_only_the_failure_with_code_1() {
    shelf("books-show-7", &["books", "show", "7"])
        .code(1)
        .stdout_eq("")
        .stderr_eq(file!["golden/books-show-7.stderr"].raw());
}

#[test]
fn books_show_prints_the_books_found_and_one_failure_naming_the_missing_ids_with_code_1() {
    shelf("books-show-2-7-9", &["books", "show", "2", "7", "9"])
        .code(1)
        .stdout_eq(file!["golden/books-show-2-7-9.txt"].raw())
        .stderr_eq(file!["golden/books-show-2-7-9.stderr"].raw());
}

#[test]
fn books_show_json_with_only_an_id_outside_the_catalogue_prints_empty_data_and_the_failure() {
    shelf("books-show-7-json", &["books", "show", "7", "--json"])
        .code(1)
        .stdout_eq(file!["golden/books-show-7-json.json"].raw())
        .stderr_eq("");
}

#[test]
fn books_show_json_prints_one_document_with_the_books_found_and_the_failure_with_code_1() {
    shelf(
        "books-show-2-7-9-json",
        &["books", "show", "2", "7", "9", "--json"],
    )
    .code(1)
    .stdout_eq(file!["golden/books-show-2-7-9-json.json"].raw())
    .stderr_eq("");
}

#[test]
fn books_show_with_101_ids_is_refused_with_code_2() {
    let mut args = vec!["books", "show"];
    args.extend(["1"; 101]);

    shelf("books-show-101-ids", &args)
        .code(2)
        .stdout_eq("")
        .stderr_eq(file!["golden/books-show-101-ids.stderr"].raw());
}

#[test]
fn books_show_without_ids_is_refused_with_code_2() {
    shelf("books-show-no-ids", &["books", "show"])
        .code(2)
        .stdout_eq("")
        .stderr_eq(file!["golden/books-show-no-ids.stderr"].raw());
}

#[test]
fn books_show_json_prints_the_books_as_an_array_and_an_empty_failures_array_without_a_total() {
    shelf("books-show-json", &["books", "show", "5", "2", "--json"])
        .success()
        .stdout_eq(file!["golden/books-show-json.json"].raw())
        .stderr_eq("");
}

#[test]
fn books_show_long_help_shows_that_ids_take_many_values_and_lists_the_output_flags() {
    shelf("books-show-help", &["books", "show", "--help"])
        .success()
        .stdout_eq(file!["golden/books-show-help.txt"].raw())
        .stderr_eq("");
}

#[test]
fn books_short_help_ends_with_the_next_line_that_opens_a_books_command() {
    shelf("books-h", &["books", "-h"])
        .success()
        .stdout_eq(file!["golden/books-h.txt"].raw())
        .stderr_eq("");
}

#[test]
fn books_without_a_command_is_refused_with_code_2() {
    shelf("books", &["books"])
        .code(2)
        .stdout_eq("")
        .stderr_eq(file!["golden/books.stderr"].raw());
}

#[test]
fn books_show_json_without_ids_prints_the_refusal_as_a_failure_document_with_code_2() {
    shelf("books-show-json-no-ids", &["books", "show", "--json"])
        .code(2)
        .stdout_eq(file!["golden/books-show-json-no-ids.json"].raw())
        .stderr_eq("");
}

#[test]
fn version_prints_shelf_and_its_version() {
    // The version holds the commit that shelf was built from, so the golden
    // file matches any text after the name.
    shelf("version", &["--version"])
        .success()
        .stdout_eq(file!["golden/version.txt"])
        .stderr_eq("");
}
