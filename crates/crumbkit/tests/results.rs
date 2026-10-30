//! A tool outside crumbkit reads what the results of its handlers hold,
//! which its own tests check, through public methods of `Listing`, `Batch`
//! and `Prose`.

use crumbkit::{Batch, Call, Code, Error, Failure, Listing, Prose, Recovery, Row};
use schemars::JsonSchema;
use serde::Serialize;

/// A book that a handler returns.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
struct Book {
    /// The book's id.
    id: u32,
}

impl Row for Book {
    const COLUMNS: &'static [&'static str] = &["id"];
}

/// The failure of an id that names no book.
#[derive(Debug)]
struct Missing(u32);

impl std::fmt::Display for Missing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "no book {}", self.0)
    }
}

impl std::error::Error for Missing {}

impl Failure for Missing {
    fn code(&self) -> &'static Code {
        const NOT_FOUND: Code = Code::new("shelf", "books", "not_found");
        &NOT_FOUND
    }

    fn recovery(&self) -> Recovery {
        Recovery::Run(Call::new(&["books", "list"]))
    }
}

/// Returns the books with the ids `ids`.
fn books(ids: &[u32]) -> Vec<Book> {
    ids.iter().map(|&id| Book { id }).collect()
}

#[test]
fn a_tool_reads_the_rows_and_the_total_of_a_listing() {
    let listing = Listing::from_all(books(&[1, 2, 3]), Some(2));

    assert_eq!(listing.rows(), books(&[1, 2]));
    assert_eq!(listing.total(), Some(3));
}

#[test]
fn a_tool_reads_the_summary_and_the_note_of_a_listing() {
    let listing = Listing::new(books(&[1]), 1)
        .note("note: one book is on loan.")
        .summary("1 book");

    assert_eq!(listing.get_summary(), Some(&"1 book"));
    assert_eq!(listing.get_note(), Some("note: one book is on loan."));
}

#[test]
fn a_listing_without_a_summary_or_a_note_has_neither() {
    let listing = Listing::new(books(&[1]), 1);

    assert!(listing.get_summary().is_none());
    assert_eq!(listing.get_note(), None);
}

#[test]
fn an_uncounted_listing_has_no_total() {
    let listing = Listing::uncounted(books(&[1]), true);

    assert_eq!(listing.total(), None);
}

#[test]
fn a_tool_reads_the_rows_of_a_batch_and_the_keys_it_leaves_out() {
    let mut batch: Batch<Book, Error> = Batch::new(&["books", "show"], "ids", &[1, 2, 3]);
    batch.push(Ok(Book { id: 1 }));
    batch.push(Ok(Book { id: 2 }));

    assert_eq!(batch.rows(), books(&[1, 2]));
    assert_eq!(batch.left_out(1), [Some("2"), Some("3")]);
}

#[test]
fn a_tool_reads_the_failures_and_the_note_of_a_batch() {
    let mut batch: Batch<Book, Missing> = Batch::new(&["books", "show"], "ids", &[1, 7]);
    batch.push(Ok(Book { id: 1 }));
    batch.push(Err(Missing(7)));
    let batch = batch.note("note: book 1 is on loan.");

    let failures: Vec<String> = batch.failures().iter().map(ToString::to_string).collect();
    assert_eq!(failures, ["no book 7"]);
    assert_eq!(batch.get_note(), Some("note: book 1 is on loan."));
}

#[test]
fn a_batch_without_a_note_has_none() {
    let batch: Batch<Book, Missing> = Batch::new(&["books", "show"], "ids", &[1]);

    assert_eq!(batch.get_note(), None);
}

#[test]
fn a_tool_reads_the_value_the_failures_and_the_note_of_a_prose_result() {
    let prose: Prose<&str, Missing> =
        Prose::with_failures("Two books.", vec![Missing(7)]).note("note: one book is on loan.");

    let failures: Vec<String> = prose.failures().iter().map(ToString::to_string).collect();
    assert_eq!(*prose.value(), "Two books.");
    assert_eq!(failures, ["no book 7"]);
    assert_eq!(prose.get_note(), Some("note: one book is on loan."));
}
