//! End-to-end tests of shelf. Each test runs the built binary and compares
//! its standard output and standard error with golden files under
//! `tests/golden/` (ADR-0004, ADR-0005). `SNAPSHOTS=overwrite cargo test`
//! rewrites the golden files.
//!
//! The `-h` and `--help` page of each command has its golden file in
//! `tests/golden/help/`, which crumbkit-test's `check_help_goldens` checks
//! from a unit test. These tests read two of those files, to prove that the
//! built binary prints the same pages.

use std::io;
use std::path::Path;

use snapbox::cmd::{Command, OutputAssert};
use snapbox::file;

/// Runs the built `shelf` binary with `args`, a cleared environment and XDG
/// directories in a temporary directory of its own, which is removed once
/// the run ends, and returns its output for assertions.
///
/// # Errors
///
/// Returns the I/O error of a temporary directory that cannot be created.
fn shelf(args: &[&str]) -> io::Result<OutputAssert> {
    let dir = tempfile::tempdir()?;
    Ok(run(dir.path(), args))
}

/// Runs shelf as [`shelf`] does, with a file named `ids.txt` that holds
/// `ids` in the temporary directory, which is also the run's working
/// directory.
///
/// # Errors
///
/// Returns the I/O error of a temporary directory or a file that cannot be
/// created.
fn shelf_with_ids_file(ids: &str, args: &[&str]) -> io::Result<OutputAssert> {
    let dir = tempfile::tempdir()?;
    #[expect(
        clippy::disallowed_methods,
        reason = "this helper owns the tests' file writes, which set up the ids file"
    )]
    std::fs::write(dir.path().join("ids.txt"), ids)?;
    Ok(run(dir.path(), args))
}

/// Runs the built `shelf` binary with `args` in the working directory `dir`,
/// with a cleared environment and XDG directories under `dir`, and returns
/// its output for assertions.
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
fn books_list_long_adds_the_form_column_and_keeps_the_footer() {
    shelf(&["books", "list", "--long"])
        .unwrap()
        .success()
        .stdout_eq(file!["golden/books-list-long.txt"].raw())
        .stderr_eq("");
}

#[test]
fn books_list_json_prints_the_compact_columns_and_the_total_as_one_json_line() {
    shelf(&["books", "list", "--json"])
        .unwrap()
        .success()
        .stdout_eq(file!["golden/books-list-json.json"].raw())
        .stderr_eq("");
}

#[test]
fn books_list_json_full_prints_every_field_of_the_first_4_books_and_the_total() {
    shelf(&["books", "list", "--json", "--full"])
        .unwrap()
        .success()
        .stdout_eq(file!["golden/books-list-json-full.json"].raw())
        .stderr_eq("");
}

#[test]
fn books_list_fields_prints_only_the_named_fields_in_order_and_the_footer() {
    shelf(&["books", "list", "--fields", "title,year"])
        .unwrap()
        .success()
        .stdout_eq(file!["golden/books-list-fields.txt"].raw())
        .stderr_eq("");
}

#[test]
fn books_list_json_fields_prints_only_the_named_fields_in_order_and_the_total() {
    shelf(&["books", "list", "--json", "--fields", "title,year"])
        .unwrap()
        .success()
        .stdout_eq(file!["golden/books-list-json-fields.json"].raw())
        .stderr_eq("");
}

#[test]
fn books_list_with_an_unknown_field_exits_with_code_2_and_hints_at_the_valid_fields() {
    shelf(&["books", "list", "--fields", "title,pages"])
        .unwrap()
        .code(2)
        .stdout_eq("")
        .stderr_eq(file!["golden/books-list-unknown-field.stderr"].raw());
}

#[test]
fn books_list_limit_2_prints_the_first_2_books_and_the_footer() {
    shelf(&["books", "list", "--limit", "2"])
        .unwrap()
        .success()
        .stdout_eq(file!["golden/books-list-limit-2.txt"].raw())
        .stderr_eq("");
}

#[test]
fn books_list_limit_0_is_refused_with_code_2() {
    shelf(&["books", "list", "--limit", "0"])
        .unwrap()
        .code(2)
        .stdout_eq("")
        .stderr_eq(file!["golden/books-list-limit-0.stderr"].raw());
}

#[test]
fn books_list_all_with_limit_is_refused_with_code_2() {
    shelf(&["books", "list", "--all", "--limit", "2"])
        .unwrap()
        .code(2)
        .stdout_eq("")
        .stderr_eq(file!["golden/books-list-all-limit-2.stderr"].raw());
}

#[test]
fn books_list_json_all_prints_every_book_and_the_total() {
    shelf(&["books", "list", "--json", "--all"])
        .unwrap()
        .success()
        .stdout_eq(file!["golden/books-list-json-all.json"].raw())
        .stderr_eq("");
}

#[test]
fn books_show_with_comma_separated_ids_is_refused_with_code_2() {
    shelf(&["books", "show", "2,5"])
        .unwrap()
        .code(2)
        .stdout_eq("")
        .stderr_eq(file!["golden/books-show-2-comma-5.stderr"].raw());
}

#[test]
fn books_show_prints_the_books_in_the_order_given() {
    shelf(&["books", "show", "5", "2"])
        .unwrap()
        .success()
        .stdout_eq(file!["golden/books-show-5-2.txt"].raw())
        .stderr_eq("");
}

#[test]
fn books_show_with_only_an_id_outside_the_catalogue_prints_only_the_failure_with_code_1() {
    shelf(&["books", "show", "7"])
        .unwrap()
        .code(1)
        .stdout_eq("")
        .stderr_eq(file!["golden/books-show-7.stderr"].raw());
}

#[test]
fn books_show_prints_the_books_found_and_one_failure_naming_the_missing_ids_with_code_1() {
    shelf(&["books", "show", "2", "7", "9"])
        .unwrap()
        .code(1)
        .stdout_eq(file!["golden/books-show-2-7-9.txt"].raw())
        .stderr_eq(file!["golden/books-show-2-7-9.stderr"].raw());
}

#[test]
fn books_show_json_with_only_an_id_outside_the_catalogue_prints_empty_data_and_the_failure() {
    shelf(&["books", "show", "7", "--json"])
        .unwrap()
        .code(1)
        .stdout_eq(file!["golden/books-show-7-json.json"].raw())
        .stderr_eq("");
}

#[test]
fn books_show_json_prints_one_document_with_the_books_found_and_the_failure_with_code_1() {
    shelf(&["books", "show", "2", "7", "9", "--json"])
        .unwrap()
        .code(1)
        .stdout_eq(file!["golden/books-show-2-7-9-json.json"].raw())
        .stderr_eq("");
}

#[test]
fn books_show_with_101_ids_is_refused_with_code_2() {
    let mut args = vec!["books", "show"];
    args.extend(["1"; 101]);

    shelf(&args)
        .unwrap()
        .code(2)
        .stdout_eq("")
        .stderr_eq(file!["golden/books-show-101-ids.stderr"].raw());
}

#[test]
fn books_show_without_ids_is_refused_with_code_2() {
    shelf(&["books", "show"])
        .unwrap()
        .code(2)
        .stdout_eq("")
        .stderr_eq(file!["golden/books-show-no-ids.stderr"].raw());
}

#[test]
fn books_show_from_a_file_prints_the_books_for_its_ids_in_order_and_skips_blank_lines() {
    shelf_with_ids_file("5\n\n  \n2\n", &["books", "show", "--from", "ids.txt"])
        .unwrap()
        .success()
        .stdout_eq(file!["golden/books-show-5-2.txt"].raw())
        .stderr_eq("");
}

#[test]
fn books_show_from_a_file_with_a_bad_line_points_at_the_line_and_exits_with_code_1() {
    shelf_with_ids_file("2\n\nfive\n5\n", &["books", "show", "--from", "ids.txt"])
        .unwrap()
        .code(1)
        .stdout_eq("")
        .stderr_eq(file!["golden/books-show-from-bad-line.stderr"].raw());
}

#[test]
fn books_show_json_from_a_file_with_a_bad_line_prints_the_failure_with_its_label() {
    shelf_with_ids_file(
        "2\n\nfive\n5\n",
        &["books", "show", "--from", "ids.txt", "--json"],
    )
    .unwrap()
    .code(1)
    .stdout_eq(file!["golden/books-show-from-bad-line-json.json"].raw())
    .stderr_eq("");
}

#[test]
fn books_show_from_a_file_that_cannot_be_read_fails_with_its_own_code_and_code_1() {
    // The cause is the operating system's message, which differs between
    // platforms, so the test matches any cause.
    shelf(&["books", "show", "--from", "missing.txt"])
        .unwrap()
        .code(1)
        .stdout_eq("")
        .stderr_eq(
            "error: cannot read the ids file missing.txt
  cause: [..]
  help:  pass --from a file that exists and that shelf can read
  code:  shelf::books::unreadable_ids_file
",
        );
}

#[test]
fn books_show_with_ids_and_from_is_refused_with_code_2() {
    shelf_with_ids_file("5\n", &["books", "show", "2", "--from", "ids.txt"])
        .unwrap()
        .code(2)
        .stdout_eq("")
        .stderr_eq(file!["golden/books-show-2-from.stderr"].raw());
}

#[test]
fn books_show_json_prints_the_books_as_an_array_and_an_empty_failures_array_without_a_total() {
    shelf(&["books", "show", "5", "2", "--json"])
        .unwrap()
        .success()
        .stdout_eq(file!["golden/books-show-json.json"].raw())
        .stderr_eq("");
}

/// crumbkit-test's `check_help_goldens` pins every help page from a unit
/// test, and this test proves that shelf's `main` prints the same root page.
#[test]
fn short_help_prints_the_root_page_that_the_help_golden_check_pins() {
    shelf(&["-h"])
        .unwrap()
        .success()
        .stdout_eq(file!["golden/help/shelf-h.txt"].raw())
        .stderr_eq("");
}

#[test]
fn books_without_a_command_is_refused_with_code_2() {
    shelf(&["books"])
        .unwrap()
        .code(2)
        .stdout_eq("")
        .stderr_eq(file!["golden/books.stderr"].raw());
}

#[test]
fn books_show_json_without_ids_prints_the_refusal_as_a_failure_document_with_code_2() {
    shelf(&["books", "show", "--json"])
        .unwrap()
        .code(2)
        .stdout_eq(file!["golden/books-show-json-no-ids.json"].raw())
        .stderr_eq("");
}

#[test]
fn help_lists_the_catalogue_topic_and_the_next_line() {
    shelf(&["help"])
        .unwrap()
        .success()
        .stdout_eq(file!["golden/help.txt"].raw())
        .stderr_eq("");
}

#[test]
fn help_catalogue_prints_the_catalogue_topic() {
    shelf(&["help", "catalogue"])
        .unwrap()
        .success()
        .stdout_eq(file!["golden/help-catalogue.txt"].raw())
        .stderr_eq("");
}

#[test]
fn help_books_show_prints_the_same_page_as_books_show_long_help() {
    shelf(&["help", "books", "show"])
        .unwrap()
        .success()
        .stdout_eq(file!["golden/help/shelf-books-show-help.txt"].raw())
        .stderr_eq("");
}

#[test]
fn version_prints_shelf_and_its_version() {
    // The version holds the commit that shelf was built from, so the golden
    // file matches any text after the name.
    shelf(&["--version"])
        .unwrap()
        .success()
        .stdout_eq(file!["golden/version.txt"])
        .stderr_eq("");
}
