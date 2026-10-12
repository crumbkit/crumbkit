//! Tests for reading as a whole that read its private items, and tests of
//! the commands that reading adds for its fixture lists. The convention
//! tests, which need only reading's public functions, live in
//! `tests/conventions.rs`.

use std::path::Path;

use clap::Command;

use crate::SPELLING;
use crate::lists::EntriesError;

/// Returns the command tree that reading builds from the lists in its
/// fixture directory.
fn fixture_command() -> Command {
    crate::command(&crate::lists_in(Path::new("tests/fixtures/lists")))
}

#[test]
fn every_failure_code_spells_uniquely_as_printable_ascii() {
    crumbkit_test::check_codes(SPELLING, &[EntriesError::CODES]);
}

#[test]
fn the_fixture_directory_adds_a_command_for_each_list() {
    let command = fixture_command();
    let names: Vec<&str> = command.get_subcommands().map(Command::get_name).collect();
    assert!(names.contains(&"finished"), "{names:?}");
    assert!(names.contains(&"to-read"), "{names:?}");
}

#[test]
fn the_command_of_a_list_has_the_about_text_of_the_list() {
    let command = fixture_command();
    let about = command
        .find_subcommand("to-read")
        .and_then(Command::get_about)
        .map(ToString::to_string);
    assert_eq!(about.as_deref(), Some("Books to read next"));
}
