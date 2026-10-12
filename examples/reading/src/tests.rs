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

/// The source of the fixture lists' names, as `lists::choices` reads the
/// lists of reading's directory.
#[expect(
    clippy::unnecessary_wraps,
    reason = "a choices source returns a `Result`"
)]
fn fixture_lists() -> Result<Vec<crumbkit::pick::Choice>, crumbkit::Error> {
    Ok(crate::lists::choices_in(Path::new("tests/fixtures/lists")))
}

#[test]
fn entries_list_without_list_asks_the_menu_and_runs_with_the_picked_list() {
    use crumbkit::help::{Guide, Leaf};
    use crumbkit::pick::Choices;

    static MENU: crumbkit_picker::Menu = crumbkit_picker::Menu::new();
    const GUIDE: Guide = Guide::new()
        .picker(&MENU)
        .leaves(&[Leaf::new(&["entries", "list"]).choices(&[Choices::new("list", fixture_lists)])]);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = crumbkit::Sink::with_input(&mut out, &mut err, &b"2\n"[..], true);
    let parsed = crumbkit::parse_to::<crate::graft::Parsed, _, _, _, _>(
        &mut sink,
        fixture_command(),
        &GUIDE,
        SPELLING,
        ["reading", "entries", "list"],
    );

    assert!(parsed.is_ok());
    let err = String::from_utf8(err).unwrap();
    assert_eq!(
        err,
        "reading entries list needs NAME: The name of the reading list, such as to-read\n\
         \x20 1. finished  Books finished this year\n\
         \x20 2. to-read  Books to read next\n\
         Type a number to pick, text to narrow the list, or nothing to cancel: \
         Running: reading entries list --list=to-read\n"
    );
}
