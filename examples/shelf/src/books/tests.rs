//! Tests for the catalogue and the calls that shelf prints.

use clap::CommandFactory as _;

use super::{show, show_call};
use crate::cli::{BooksShow, Cli};

#[test]
fn every_call_that_shelf_prints_resolves_in_its_command_tree() {
    let rendered = show_call().render(&mut Cli::command()).unwrap();

    assert_eq!(rendered, "shelf books show <ID>...");
}

#[test]
fn show_returns_the_books_in_the_order_given_and_repeats_a_repeated_id() {
    let args = BooksShow { ids: vec![5, 2, 5] };

    let titles: Vec<&str> = show(&args).iter().map(|book| book.title).collect();

    assert_eq!(titles, ["Middlemarch", "Frankenstein", "Middlemarch"]);
}
