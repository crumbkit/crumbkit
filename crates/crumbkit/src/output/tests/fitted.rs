//! Tests for how `finish` fits a text table to the width of the terminal
//! that standard output is (ADR-0121).

use clap::CommandFactory as _;
use schemars::JsonSchema;
use serde::Serialize;

use super::{Finished, Tool, built_with, finish_output};
use crate::help::Guide;
use crate::{Document, Listing, Output, Row};

/// A book whose `isbn` is an identifier that a person copies, so the fit
/// never cuts it.
#[derive(Serialize, JsonSchema)]
struct Book {
    /// The book's ISBN.
    isbn: &'static str,
    /// The book's title.
    title: String,
    /// The book's author.
    author: &'static str,
}

impl Row for Book {
    const COLUMNS: &'static [&'static str] = &["isbn", "title"];
    const LONG_COLUMNS: &'static [&'static str] = &["author"];
    const UNCUT_COLUMNS: &'static [&'static str] = &["isbn"];
}

const BOOKS: Document<Vec<Book>> = Document::new("library", "books", 1, "The test rows.");
const BOOK_LISTING: Document<Listing<Book>> =
    Document::new("library", "book-listing", 1, "The test rows.");

/// Returns two books with long titles.
fn books() -> Vec<Book> {
    vec![
        Book {
            isbn: "978-0141439518",
            title: "Pride and Prejudice".to_owned(),
            author: "Jane Austen",
        },
        Book {
            isbn: "978-0486424538",
            title: "The Adventures of Sherlock Holmes".to_owned(),
            author: "Arthur Conan Doyle",
        },
    ]
}

/// Returns the output of `tool list` with `flags` and `guide`, at a
/// terminal `width` columns wide when `width` is set.
fn at_terminal(guide: &Guide, flags: &[&str], width: Option<usize>) -> Output {
    let args: Vec<&str> = ["tool", "list"].iter().chain(flags).copied().collect();
    let mut output = built_with(Tool::command(), guide, &args);
    output.terminal_width = width;
    output
}

/// Finishes the two books with `flags` at a terminal `width` columns wide.
fn finish_books(flags: &[&str], width: Option<usize>) -> Finished {
    finish_output(at_terminal(&Guide::new(), flags, width), BOOKS, books())
}

#[test]
fn a_compact_table_at_a_narrow_terminal_cuts_its_widest_column_to_fit() {
    let finished = finish_books(&[], Some(31));

    let expected = "\
ISBN            TITLE
978-0141439518  Pride and Prej…
978-0486424538  The Adventures…
";
    assert_eq!(finished.out, expected);
    assert_eq!(finished.err, "");
}

#[test]
fn a_long_table_at_a_narrow_terminal_fits_and_keeps_the_uncut_column_whole() {
    let finished = finish_books(&["--long"], Some(44));

    let expected = "\
ISBN            TITLE     AUTHOR
978-0141439518  Pride a…  Jane Austen
978-0486424538  The Adv…  Arthur Conan Doyle
";
    assert_eq!(finished.out, expected);
}

#[test]
fn a_fields_table_fits_and_keeps_a_column_named_in_uncut_columns_whole() {
    let finished = finish_books(&["--fields", "title,isbn"], Some(31));

    let expected = "\
TITLE            ISBN
Pride and Prej…  978-0141439518
The Adventures…  978-0486424538
";
    assert_eq!(finished.out, expected);
}

#[test]
fn json_at_a_narrow_terminal_prints_the_same_bytes_as_without_a_terminal() {
    let narrow = finish_books(&["--json"], Some(20));
    let piped = finish_books(&["--json"], None);

    assert_eq!(narrow.out, piped.out);
    assert!(
        narrow.out.contains("The Adventures of Sherlock Holmes"),
        "{}",
        narrow.out
    );
}

#[test]
fn the_output_budget_shows_the_same_rows_with_and_without_a_terminal_width() {
    let guide = Guide::new().output_budget(1024, "the tests print little");
    let listing = || {
        let rows: Vec<Book> = (0..30)
            .map(|number| Book {
                isbn: "978-0141439518",
                title: format!("{number:02} {}", "a".repeat(60)),
                author: "Jane Austen",
            })
            .collect();
        Listing::new(rows, 30).described("books")
    };

    let narrow = finish_output(at_terminal(&guide, &[], Some(40)), BOOK_LISTING, listing());
    let piped = finish_output(at_terminal(&guide, &[], None), BOOK_LISTING, listing());

    assert_ne!(narrow.out, piped.out, "the narrow table was not fitted");
    assert_eq!(narrow.out.lines().count(), piped.out.lines().count());
    assert_eq!(narrow.err, piped.err);
    assert!(
        piped.err.starts_with("Showing "),
        "the budget cut no row: {}",
        piped.err
    );
}
