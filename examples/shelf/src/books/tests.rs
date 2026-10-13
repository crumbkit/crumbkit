//! Tests for the catalogue, its failures and the calls that shelf prints.

use clap::CommandFactory as _;
use crumbkit::{Failure as _, Recovery, RenderCall as _, Rows as _};

use super::{BooksError, IdsFileProblem, parse_ids, show, show_call, summarize};
use crate::cli::{BooksShow, BooksSummarize, Cli};

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

    let batch = show(&args, None, usize::MAX).unwrap();

    let titles: Vec<&str> = batch.rows().iter().map(|book| book.title).collect();
    assert_eq!(titles, ["Middlemarch", "Frankenstein", "Middlemarch"]);
    assert_eq!(batch.failures().unwrap().len(), 0);
}

#[test]
fn show_returns_one_failure_naming_one_missing_id_as_a_book_and_no_books() {
    let args = BooksShow {
        ids: vec![7],
        from: None,
    };

    let batch = show(&args, None, usize::MAX).unwrap();

    let failures: Vec<String> = batch
        .failures()
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert!(batch.rows().is_empty());
    assert_eq!(failures, ["the catalogue has no book 7"]);
}

#[test]
fn show_returns_the_books_found_and_one_failure_for_each_missing_id_in_the_order_given() {
    let args = BooksShow {
        ids: vec![2, 9, 7, 9],
        from: None,
    };

    let batch = show(&args, None, usize::MAX).unwrap();

    let titles: Vec<&str> = batch.rows().iter().map(|book| book.title).collect();
    let failures: Vec<String> = batch
        .failures()
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect();
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
        ids: vec![1, 2, 3],
        from: None,
    };

    let batch = show(&args, None, 1).unwrap();

    assert_eq!(batch.rows().len(), 1);
    assert_eq!(batch.left_out(1), [Some("2"), Some("3")]);
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
    let text = prose.prose().unwrap().to_string();
    let failures = prose
        .failures()
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect();
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
