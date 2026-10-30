//! Tests for the catalogue, its failures and the calls that shelf prints.

use clap::CommandFactory as _;
use crumbkit::help::Guide;
use crumbkit::{Batch, Failure as _, Output, Recovery, RenderCall as _, Room, ViewFlags};

use super::{
    BOOKS_SHOW, Book, BooksError, IdsFileProblem, MOST_IDS, parse_ids, show, show_call, summarize,
};
use crate::cli::{BooksShow, BooksSummarize, Cli};

/// Returns the room of `books show` with crumbkit's default output budget.
fn roomy() -> Room<Batch<Book, BooksError>> {
    Output::new(
        ViewFlags::new(),
        crumbkit::DryRun::default(),
        Cli::command(),
        &Guide::new(),
        ["shelf"],
    )
    .unwrap()
    .room(BOOKS_SHOW)
}

#[test]
fn every_call_that_shelf_prints_resolves_in_its_command_tree() {
    let error = BooksError::NotFound { ids: vec![7] };
    let Recovery::Run(recovery) = error.recovery() else {
        panic!("a missing book's recovery runs a command");
    };

    let next = show_call().render(&mut Cli::command()).unwrap();
    let recovery = recovery.render(&mut Cli::command()).unwrap();

    assert_eq!(next, "shelf books show <ID>...");
    assert_eq!(recovery, "shelf books list");
}

#[test]
fn show_returns_the_books_in_the_order_given_and_repeats_a_repeated_id() {
    let args = BooksShow {
        ids: vec![5, 2, 5],
        from: None,
    };

    let batch = show(&args, None, roomy()).unwrap();

    let titles: Vec<&str> = batch.rows().iter().map(|book| book.title).collect();
    assert_eq!(titles, ["Middlemarch", "Frankenstein", "Middlemarch"]);
    assert_eq!(batch.failures().len(), 0);
}

#[test]
fn show_returns_one_failure_naming_one_missing_id_as_a_book_and_no_books() {
    let args = BooksShow {
        ids: vec![7],
        from: None,
    };

    let batch = show(&args, None, roomy()).unwrap();

    let failures: Vec<String> = batch.failures().iter().map(ToString::to_string).collect();
    assert!(batch.rows().is_empty());
    assert_eq!(failures, ["the catalogue has no book 7"]);
}

#[test]
fn show_returns_the_books_found_and_one_failure_for_each_missing_id_in_the_order_given() {
    let args = BooksShow {
        ids: vec![2, 9, 7, 9],
        from: None,
    };

    let batch = show(&args, None, roomy()).unwrap();

    let titles: Vec<&str> = batch.rows().iter().map(|book| book.title).collect();
    let failures: Vec<String> = batch.failures().iter().map(ToString::to_string).collect();
    assert_eq!(titles, ["Frankenstein"]);
    assert_eq!(
        failures,
        [
            "the catalogue has no book 9",
            "the catalogue has no book 7",
            "the catalogue has no book 9"
        ]
    );
}

#[test]
fn show_stops_looking_up_ids_once_its_books_fill_the_budget() {
    let args = BooksShow {
        ids: vec![1; MOST_IDS],
        from: None,
    };
    let guide = Guide::new().output_budget(1024, "the test fills it");
    let words = ["shelf", "books", "show", "1"];
    let (_, output) = crumbkit::parse_from::<Cli, _, _>(Cli::command(), &guide, words).unwrap();

    let batch = show(&args, None, output.room(BOOKS_SHOW)).unwrap();

    let shown = batch.rows().len();
    assert!((1..MOST_IDS).contains(&shown), "{shown}");
    assert_eq!(batch.left_out(shown).len(), MOST_IDS - shown);
}

#[test]
fn parse_ids_points_at_a_bad_id_without_the_spaces_around_it() {
    let problem = parse_ids(&mut "1\r\n  x2 \n3\n".as_bytes(), &mut String::new()).unwrap_err();

    let IdsFileProblem::BadLine { span, reason } = problem else {
        panic!("a line that is not an id is a bad line, and parse_ids returned {problem:?}");
    };
    assert_eq!((span.offset(), span.len()), (5, 2));
    assert_eq!(reason, "invalid digit found in string");
}

#[test]
fn parse_ids_stops_reading_at_the_line_after_id_101() {
    let lines = "1\n".repeat(103);
    let mut reader = lines.as_bytes();
    let mut text = String::new();

    let problem = parse_ids(&mut reader, &mut text).unwrap_err();

    assert!(
        matches!(problem, IdsFileProblem::TooMany { .. }),
        "{problem:?}"
    );
    assert_eq!(text, "1\n".repeat(102));
    assert_eq!(reader, b"1\n");
}

/// Returns the text and the failures of `shelf books summarize` for `ids`.
fn summarize_ids(ids: &[u32]) -> (String, Vec<String>) {
    let args = BooksSummarize { ids: ids.to_vec() };
    let prose = summarize(&args, None).unwrap();
    let text = prose.value().to_string();
    let failures = prose.failures().iter().map(ToString::to_string).collect();
    (text, failures)
}

#[test]
fn summarize_without_ids_describes_the_whole_catalogue_in_one_sentence() {
    let (text, failures) = summarize_ids(&[]);

    assert_eq!(
        text,
        "6 books by 6 authors, first published from 1813 to 1892: 5 novels and 1 collection of short stories."
    );
    assert_eq!(failures.len(), 0);
}

#[test]
fn summarize_of_one_book_uses_the_singular_and_names_one_year() {
    let (text, _) = summarize_ids(&[2]);

    assert_eq!(
        text,
        "1 book by 1 author, first published in 1818: 1 novel."
    );
}

#[test]
fn summarize_counts_a_repeated_id_once_and_returns_one_failure_naming_each_missing_id_once() {
    let (text, failures) = summarize_ids(&[6, 7, 6, 9, 7]);

    assert_eq!(
        text,
        "1 book by 1 author, first published in 1892: 1 collection of short stories."
    );
    assert_eq!(failures, ["the catalogue has no books 7, 9"]);
}

#[test]
fn summarize_of_only_missing_ids_says_that_it_found_no_books() {
    let (text, failures) = summarize_ids(&[7]);

    assert_eq!(text, "No books.");
    assert_eq!(failures, ["the catalogue has no book 7"]);
}

#[test]
fn summarize_serializes_its_counts_and_the_years_of_the_books_it_found() {
    let args = BooksSummarize { ids: vec![2, 5] };

    let prose = summarize(&args, None).unwrap();

    assert_eq!(
        serde_json::to_value(&prose).unwrap(),
        serde_json::json!({
            "books": 2,
            "authors": 2,
            "earliest": 1818,
            "latest": 1871,
            "novels": 2,
            "short_story_collections": 0
        })
    );
}

/// Returns a file of removed books that holds `ids`, kept in a temporary
/// directory that the caller holds.
fn removed_books(ids: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("removed.txt");
    #[expect(
        clippy::disallowed_methods,
        reason = "this helper owns the tests' file writes, which set up the file of removed books"
    )]
    std::fs::write(&path, ids).unwrap();
    (dir, path)
}

#[test]
fn show_notes_which_of_the_missing_ids_name_books_removed_from_the_shelf() {
    let (_dir, removed) = removed_books("2\n5\n");
    let args = BooksShow {
        ids: vec![1, 2, 7, 5],
        from: None,
    };

    let batch = show(&args, Some(&removed), roomy()).unwrap();

    assert_eq!(
        batch.get_note(),
        Some("note: books 2, 5 are in the catalogue but were removed from the shelf.")
    );
}

#[test]
fn show_has_no_note_when_no_id_names_a_removed_book() {
    let (_dir, removed) = removed_books("2\n");
    let args = BooksShow {
        ids: vec![1, 7],
        from: None,
    };

    let batch = show(&args, Some(&removed), roomy()).unwrap();

    assert_eq!(batch.get_note(), None);
}

#[test]
fn summarize_without_ids_notes_the_removed_books_it_leaves_out() {
    let (_dir, removed) = removed_books("2\n");
    let args = BooksSummarize { ids: Vec::new() };

    let prose = summarize(&args, Some(&removed)).unwrap();

    assert_eq!(
        prose.value().to_string(),
        "5 books by 5 authors, first published from 1813 to 1892: 4 novels and 1 collection of short stories."
    );
    assert_eq!(
        prose.get_note(),
        Some("note: the summary leaves out book 2, which was removed from the shelf.")
    );
}

#[test]
fn summarize_has_no_note_when_no_book_was_removed() {
    let args = BooksSummarize { ids: Vec::new() };

    let prose = summarize(&args, None).unwrap();

    assert_eq!(prose.get_note(), None);
}
