//! Tests for the catalogue, its failures and the calls that shelf prints.

use clap::CommandFactory as _;
use crumbkit::{Failure as _, Recovery};

use super::{BooksError, show, show_call};
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
    let args = BooksShow { ids: vec![5, 2, 5] };

    let books = show(&args).unwrap();

    let titles: Vec<&str> = books.iter().map(|book| book.title).collect();
    assert_eq!(titles, ["Middlemarch", "Frankenstein", "Middlemarch"]);
}

#[test]
fn show_fails_naming_one_missing_id_as_a_book() {
    let args = BooksShow { ids: vec![7] };

    let error = show(&args).err().unwrap();

    assert_eq!(error.to_string(), "the catalogue has no book 7");
}

#[test]
fn show_fails_as_a_whole_naming_each_missing_id_once_in_the_order_given() {
    let args = BooksShow {
        ids: vec![2, 9, 7, 9],
    };

    let error = show(&args).err().unwrap();

    assert_eq!(error.to_string(), "the catalogue has no books 9, 7");
}
