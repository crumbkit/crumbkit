//! Tests for the catalogue, its failures and the calls that shelf prints.

use clap::CommandFactory as _;
use crumbkit::{Failure as _, Recovery, Rows as _};

use super::{BooksError, parse_ids, show, show_call};
use crate::cli::{BooksShow, Cli};

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

    let batch = show(&args).unwrap();

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

    let batch = show(&args).unwrap();

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
fn show_returns_the_books_found_and_one_failure_naming_each_missing_id_once_in_the_order_given() {
    let args = BooksShow {
        ids: vec![2, 9, 7, 9],
        from: None,
    };

    let batch = show(&args).unwrap();

    let titles: Vec<&str> = batch.rows().iter().map(|book| book.title).collect();
    let failures: Vec<String> = batch
        .failures()
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(titles, ["Frankenstein"]);
    assert_eq!(failures, ["the catalogue has no books 9, 7"]);
}

#[test]
fn parse_ids_points_at_a_bad_id_without_the_spaces_around_it() {
    let (span, problem) = parse_ids("1\r\n  x2 \n3\n").unwrap_err();

    assert_eq!((span.offset(), span.len()), (5, 2));
    assert_eq!(problem, "invalid digit found in string");
}
