//! Tests for listings.

use schemars::JsonSchema;
use serde::Serialize;
use serde_json::json;

use super::Listing;
use crate::{Call, Row, Rows};

#[derive(Serialize, JsonSchema, Debug, Clone, PartialEq, Eq)]
struct Book {
    id: u32,
}

impl Row for Book {
    const COLUMNS: &'static [&'static str] = &["id"];
}

/// Returns the books with the ids 1 to `count`.
fn books(count: u32) -> Vec<Book> {
    (1..=count).map(|id| Book { id }).collect()
}

#[test]
fn new_keeps_the_rows_and_the_total_it_is_given() {
    let listing = Listing::new(books(2), 41);

    assert_eq!(listing.rows(), books(2));
    assert_eq!(listing.total(), Some(41));
}

#[test]
fn a_total_smaller_than_the_rows_becomes_the_number_of_rows() {
    let listing = Listing::new(books(3), 1);

    assert_eq!(listing.total(), Some(3));
}

#[test]
fn from_all_with_a_limit_below_the_length_keeps_the_first_rows_and_counts_them_all() {
    let listing = Listing::from_all(books(6), Some(4));

    assert_eq!(listing, Listing::new(books(4), 6));
}

#[test]
fn from_all_with_a_limit_equal_to_the_length_keeps_every_row() {
    let listing = Listing::from_all(books(6), Some(6));

    assert_eq!(listing, Listing::new(books(6), 6));
}

#[test]
fn from_all_with_a_limit_above_the_length_keeps_every_row() {
    let listing = Listing::from_all(books(6), Some(10));

    assert_eq!(listing, Listing::new(books(6), 6));
}

#[test]
fn from_all_without_a_limit_keeps_every_row() {
    let listing = Listing::from_all(books(6), None);

    assert_eq!(listing, Listing::new(books(6), 6));
}

#[test]
fn a_listing_has_no_description_until_one_is_set() {
    let plain = Listing::new(books(1), 3);
    let described = Listing::new(books(1), 3).described("books by author 4");

    assert_eq!(plain.description(), None);
    assert_eq!(described.description(), Some("books by author 4"));
}

#[test]
fn a_listing_has_no_next_step_until_one_is_set() {
    let plain = Listing::new(books(1), 3);
    let call = Call::new(&["books", "show"]).arg("ids");
    let with_next = Listing::new(books(1), 3).next(call.clone(), "takes many ids.");

    assert_eq!(plain.next_step(), None);
    assert_eq!(with_next.next_step(), Some((&call, "takes many ids.")));
}

#[test]
fn a_listing_serializes_as_the_array_of_its_rows() {
    let listing = Listing::new(books(2), 41).described("books");

    assert_eq!(
        serde_json::to_value(&listing).unwrap(),
        json!([{"id": 1}, {"id": 2}])
    );
}

#[test]
fn a_single_row_and_a_vec_have_no_total_no_description_and_no_next_step() {
    let book = Book { id: 1 };

    assert_eq!(book.total(), None);
    assert_eq!(book.description(), None);
    assert_eq!(book.next_step(), None);
    assert_eq!(books(2).total(), None);
    assert_eq!(books(2).description(), None);
    assert_eq!(books(2).next_step(), None);
}

/// A summary of the books: how many there are, which prints as a line.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
struct Shelved {
    books: u32,
}

impl std::fmt::Display for Shelved {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} books on the shelf.", self.books)
    }
}

#[test]
fn a_listing_has_no_summary_until_one_is_set() {
    let plain = Listing::new(books(1), 3);
    let summarised = Listing::new(books(1), 3)
        .described("books")
        .summary(Shelved { books: 3 });

    // The builder `summary` shadows the `Rows` method of the same name.
    assert_eq!(Rows::summary(&plain), None);
    assert_eq!(Rows::summary(&summarised), Some(&Shelved { books: 3 }));
    assert_eq!(summarised.description(), Some("books"));
}
