//! End-to-end tests of the document that each list's `list` command
//! declares, the one that `entries list` prints. Each test runs the built
//! binary with `READING_DIR` set to the fixture lists and reads what help,
//! `schemas list` and `commands list` say about the command.

use std::io;

use snapbox::cmd::Command;

/// The directory of reading's fixture lists.
const LISTS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/lists");

/// Runs the built `reading` binary with `args`, `READING_DIR` set to the
/// fixture lists, a cleared environment and XDG directories in a temporary
/// directory of its own, and returns its standard output after checking
/// that the run succeeded.
///
/// # Errors
///
/// Returns the I/O error of a temporary directory that cannot be created.
fn stdout(args: &[&str]) -> io::Result<String> {
    let xdg = tempfile::tempdir()?;
    let output = Command::new(env!("CARGO_BIN_EXE_reading"))
        .args(args)
        .env_clear()
        .env("READING_DIR", LISTS)
        .env("XDG_CONFIG_HOME", xdg.path().join("config"))
        .env("XDG_DATA_HOME", xdg.path().join("data"))
        .env("XDG_STATE_HOME", xdg.path().join("state"))
        .env("XDG_CACHE_HOME", xdg.path().join("cache"))
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    Ok(String::from_utf8_lossy(&output).into_owned())
}

#[test]
fn a_list_commands_help_lists_the_columns_of_the_entries_list_document() -> io::Result<()> {
    let short = stdout(&["to-read", "list", "-h"])?;
    assert!(
        short.contains("Next: reading to-read list --help explains each column.\n"),
        "{short}"
    );
    let long = stdout(&["to-read", "list", "--help"])?;
    assert!(
        long.contains(
            "Columns:\n  position  The entry's position in the list, counting from 1.\n  title     The title of the book.\n"
        ),
        "{long}"
    );
    Ok(())
}

#[test]
fn schemas_list_names_each_list_command_among_the_commands_that_print_entries_list()
-> io::Result<()> {
    let stdout = stdout(&["schemas", "list", "--json", "--full"])?;
    assert!(
        stdout.contains(
            r#""id":"reading/entries-list/v1","kind":"output","form":"document","description":"One page of a reading list's entries.","commands":["reading entries list","reading finished list","reading to-read list"]"#
        ),
        "{stdout}"
    );
    Ok(())
}

#[test]
fn commands_list_gives_a_list_command_the_columns_and_form_of_entries_list() -> io::Result<()> {
    let stdout = stdout(&["commands", "list", "to-read", "--json", "--full"])?;
    assert!(
        stdout.contains(
            r#""columns":[{"name":"position","long":false,"description":"The entry's position in the list, counting from 1."},{"name":"title","long":false,"description":"The title of the book."}],"form":"document""#
        ),
        "{stdout}"
    );
    Ok(())
}
