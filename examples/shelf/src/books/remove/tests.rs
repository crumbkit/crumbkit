//! Tests for `shelf books remove` and the file of removed books.

use std::path::Path;

use clap::{CommandFactory as _, Parser as _};
use crumbkit::Output;
use crumbkit::help::Guide;

use super::{read_removed, remove, removed_file_in};
use crate::books::{BooksError, list, show};
use crate::cli::{BooksCommand, BooksList, BooksRemove, BooksShow, Cli, Resource};

/// Parses `shelf books remove` with `words` after `remove`, and returns its
/// output flags and arguments.
fn parse(words: &[&str]) -> (Output, BooksRemove) {
    let args: Vec<&str> = ["shelf", "books", "remove"]
        .iter()
        .chain(words)
        .copied()
        .collect();
    let cli = Cli::try_parse_from(&args).unwrap();
    let Resource::Books {
        command: BooksCommand::Remove(remove),
    } = cli.resource
    else {
        panic!("the words parse as books remove");
    };
    let output = Output::new(cli.view, Cli::command(), &Guide::new(), args).unwrap();
    (output, remove)
}

/// Returns the titles of the books that `shelf books list --all` prints with
/// the removed books in `file`.
fn listed(file: &Path) -> Vec<&'static str> {
    let cli = Cli::try_parse_from(["shelf", "books", "list", "--all"]).unwrap();
    let Resource::Books {
        command: BooksCommand::List(args),
    } = cli.resource
    else {
        panic!("the words parse as books list");
    };
    let args: BooksList = args;
    let listing = list(&args, Some(file)).unwrap();
    crumbkit::Rows::rows(&listing)
        .iter()
        .map(|book| book.title)
        .collect()
}

#[test]
fn remove_with_yes_returns_the_books_and_list_and_show_skip_them_afterwards() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("shelf").join("removed.txt");
    let (mut output, args) = parse(&["2", "5", "--yes"]);

    let books = remove(&mut output, &args, Some(&file)).unwrap();

    let titles: Vec<&str> = books.iter().map(|book| book.title).collect();
    assert_eq!(titles, ["Frankenstein", "Middlemarch"]);
    assert_eq!(read_removed(Some(&file)).unwrap(), [2, 5]);
    assert_eq!(
        listed(&file),
        [
            "Pride and Prejudice",
            "Moby-Dick",
            "Great Expectations",
            "The Adventures of Sherlock Holmes"
        ]
    );
    let shown = show(
        &BooksShow {
            ids: vec![2],
            from: None,
        },
        Some(&file),
        Output::new(
            crumbkit::ViewFlags::new(),
            Cli::command(),
            &Guide::new(),
            ["shelf"],
        )
        .unwrap()
        .room(crate::books::BOOKS_SHOW),
    )
    .unwrap();
    let failures: Vec<String> = crumbkit::Rows::failures(&shown)
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(failures, ["the catalogue has no book 2"]);
}

#[test]
fn a_dry_run_returns_the_books_and_writes_no_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("removed.txt");
    let (mut output, args) = parse(&["2", "--dry-run"]);

    let books = remove(&mut output, &args, Some(&file)).unwrap();

    assert_eq!(books.len(), 1);
    assert!(!file.exists());
}

#[test]
fn removing_a_removed_book_again_succeeds_and_keeps_one_entry_for_it() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("removed.txt");
    let (mut output, args) = parse(&["2", "2", "--yes"]);
    remove(&mut output, &args, Some(&file)).unwrap();

    let books = remove(&mut output, &args, Some(&file)).unwrap();

    assert_eq!(books.len(), 1);
    assert_eq!(read_removed(Some(&file)).unwrap(), [2]);
}

#[test]
fn an_id_outside_the_catalogue_fails_with_not_found_and_removes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("removed.txt");
    let (mut output, args) = parse(&["2", "7", "--yes"]);

    let Err(error) = remove(&mut output, &args, Some(&file)) else {
        panic!("removing an id outside the catalogue fails");
    };

    assert_eq!(error.to_string(), "the catalogue has no book 7");
    assert!(!file.exists());
}

#[test]
fn without_a_state_directory_remove_fails_and_list_skips_nothing() {
    let (mut output, args) = parse(&["2", "--yes"]);

    let Err(error) = remove(&mut output, &args, None) else {
        panic!("removing without a state directory fails");
    };

    assert_eq!(
        error.to_string(),
        "shelf cannot find a state directory to keep the removed books in"
    );
    assert_eq!(read_removed(None).unwrap(), Vec::<u32>::new());
}

#[test]
fn a_file_of_removed_books_with_a_line_that_is_not_an_id_fails_to_read() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("removed.txt");
    #[expect(
        clippy::disallowed_methods,
        reason = "the test sets up a damaged file of removed books"
    )]
    std::fs::write(&file, "2\nx\n").unwrap();

    let error = read_removed(Some(&file)).unwrap_err();

    assert!(matches!(error, BooksError::RemovedFile { .. }), "{error:?}");
}

/// An absolute state directory on this platform.
#[cfg(unix)]
const ABSOLUTE_STATE: &str = "/srv/state";
/// An absolute state directory on this platform.
#[cfg(windows)]
const ABSOLUTE_STATE: &str = r"C:\srv\state";

#[test]
fn an_absolute_xdg_state_home_names_the_state_directory_of_the_removed_file() {
    let file = removed_file_in(Some(ABSOLUTE_STATE.into()));

    assert_eq!(
        file,
        Some(Path::new(ABSOLUTE_STATE).join("shelf").join("removed.txt"))
    );
}

#[test]
fn a_relative_or_missing_xdg_state_home_leaves_the_removed_file_to_etcetera() {
    use etcetera::BaseStrategy as _;

    let etcetera = etcetera::choose_base_strategy()
        .ok()
        .and_then(|strategy| strategy.state_dir())
        .map(|state| state.join("shelf").join("removed.txt"));

    assert_eq!(removed_file_in(Some("state".into())), etcetera);
    assert_eq!(removed_file_in(None), etcetera);
}
