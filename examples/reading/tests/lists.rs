//! End-to-end tests of the commands that reading adds for its lists. Each
//! test runs the built binary with `READING_DIR` set to the fixture lists
//! and compares its output with a golden file under `tests/golden/`
//! (ADR-0004, ADR-0005). `SNAPSHOTS=overwrite cargo test` rewrites the
//! golden files.

use std::io;

use snapbox::cmd::{Command, OutputAssert};
use snapbox::file;

/// The directory of reading's fixture lists.
const LISTS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/lists");

/// Runs the built `reading` binary with `args`, `READING_DIR` set to `dir`, a
/// cleared environment and XDG directories in a temporary directory of its
/// own, and returns its output for assertions.
///
/// # Errors
///
/// Returns the I/O error of a temporary directory that cannot be created.
fn reading(dir: &str, args: &[&str]) -> io::Result<OutputAssert> {
    let xdg = tempfile::tempdir()?;
    Ok(Command::new(env!("CARGO_BIN_EXE_reading"))
        .args(args)
        .env_clear()
        .env("READING_DIR", dir)
        .env("XDG_CONFIG_HOME", xdg.path().join("config"))
        .env("XDG_DATA_HOME", xdg.path().join("data"))
        .env("XDG_STATE_HOME", xdg.path().join("state"))
        .env("XDG_CACHE_HOME", xdg.path().join("cache"))
        .assert())
}

#[test]
fn a_list_command_prints_the_entries_of_its_list_as_json() -> io::Result<()> {
    reading(LISTS, &["to-read", "list", "--json", "--limit", "2"])?
        .success()
        .stdout_eq(file!["golden/to-read-list-json-limit-2.json"]);
    Ok(())
}

#[test]
fn entries_list_names_the_list_to_print() -> io::Result<()> {
    reading(LISTS, &["entries", "list", "--list", "finished"])?
        .success()
        .stdout_eq(file!["golden/entries-list-finished.txt"]);
    Ok(())
}

#[test]
fn entries_list_of_a_list_that_does_not_exist_fails_with_a_next_step() -> io::Result<()> {
    reading(LISTS, &["entries", "list", "--list", "unread"])?
        .code(1)
        .stdout_eq("")
        .stderr_eq(file!["golden/entries-list-unread.stderr"]);
    Ok(())
}

#[test]
fn a_directory_without_lists_adds_no_command() -> io::Result<()> {
    let empty = tempfile::tempdir()?;
    let dir = empty.path().to_string_lossy().into_owned();
    reading(&dir, &["to-read", "list"])?
        .code(2)
        .stdout_eq("")
        .stderr_eq(file!["golden/to-read-without-lists.stderr"]);
    Ok(())
}
