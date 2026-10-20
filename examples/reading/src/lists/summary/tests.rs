//! Tests for `reading lists list`'s `updated` column, which shows the age of
//! each list's file. Each test passes a fixed current time, so the age does
//! not depend on when the test runs.

use std::io;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use clap::{Args as _, FromArgMatches as _};
use crumbkit::{Listing, Page, Sink};

use super::{LISTS_LIST, ListSummary, summaries, summary};
use crate::graft::Parsed;
use crate::lists::{List, lists_in};

/// The current time that the tests pass, 2026-10-07T12:01:07.120Z.
fn now() -> SystemTime {
    UNIX_EPOCH + Duration::from_millis(1_791_374_467_120)
}

/// Returns the page of `reading lists list` without `--limit` or `--all`.
fn page() -> Page<10> {
    let command = Page::<10>::augment_args(clap::Command::new("reading"));
    let matches = command.try_get_matches_from(["reading"]).unwrap();
    Page::<10>::from_arg_matches(&matches).unwrap()
}

/// Writes the list `to-read` into `dir` and sets the time its file was last
/// modified to `modified`.
///
/// # Errors
///
/// Returns the I/O error of a file that cannot be written or whose time
/// cannot be set.
fn write_list(dir: &Path, modified: SystemTime) -> io::Result<()> {
    let path = dir.join("to-read.txt");
    #[expect(
        clippy::disallowed_methods,
        reason = "the test writes its own list into a temporary directory that it owns"
    )]
    let file = {
        std::fs::write(&path, "# Books to read next\nEmma\n")?;
        std::fs::File::options().write(true).open(&path)?
    };
    file.set_modified(modified)
}

/// Prints `listing` as `reading lists list` followed by `flags` prints it,
/// and returns what it printed on standard output.
fn print(listing: Listing<ListSummary>, flags: &[&str]) -> String {
    let lists = lists_in(Path::new("tests/fixtures/lists"));
    let args = ["reading", "lists", "list"].iter().chain(flags);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);
    let parsed = crumbkit::parse_to::<Parsed, _, _, _, _>(
        &mut sink,
        crate::command(&lists),
        &crate::guide(&lists),
        args,
    );
    let Ok((_parsed, output)) = parsed else {
        panic!("the line did not parse");
    };
    let code = crumbkit::finish_to(
        &mut sink,
        output,
        LISTS_LIST,
        Ok::<_, crate::lists::EntriesError>(listing),
    );
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&err));
    String::from_utf8(out).unwrap()
}

#[test]
fn the_updated_column_shows_the_age_of_the_lists_file_in_text() -> io::Result<()> {
    let dir = tempfile::tempdir()?;
    write_list(dir.path(), now() - Duration::from_secs(3 * 3_600))?;

    let listing = summaries(Some(dir.path()), &page(), now());

    let expected = "\
NAME     UPDATED
to-read  3h
";
    assert_eq!(print(listing, &["--fields", "name,updated"]), expected);
    Ok(())
}

#[test]
fn the_updated_field_holds_the_time_the_lists_file_was_modified_in_json() -> io::Result<()> {
    let dir = tempfile::tempdir()?;
    write_list(dir.path(), now() - Duration::from_secs(3 * 3_600))?;

    let listing = summaries(Some(dir.path()), &page(), now());

    let json = print(listing, &["--json"]);
    assert!(
        json.contains(r#""updated":"2026-10-07T09:01:07.120Z""#),
        "{json}"
    );
    Ok(())
}

#[test]
fn the_text_line_of_a_list_leaves_out_its_age() -> io::Result<()> {
    let dir = tempfile::tempdir()?;
    write_list(dir.path(), now() - Duration::from_secs(3 * 3_600))?;

    let listing = summaries(Some(dir.path()), &page(), now());

    assert_eq!(print(listing, &[]), "to-read: Books to read next\n");
    Ok(())
}

#[test]
fn a_list_whose_modification_time_is_unknown_shows_a_dash_in_text_and_null_in_json() {
    let list = || summary(List::new("kept".to_owned(), "Kept".to_owned()), now());
    let listing = || page().first_of(vec![list()]);

    let text = print(listing(), &["--fields", "name,updated"]);
    let json = print(listing(), &["--json"]);

    assert_eq!(text, "NAME  UPDATED\nkept  -\n");
    assert!(json.contains(r#""updated":null"#), "{json}");
}
