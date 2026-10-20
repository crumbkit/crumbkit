//! Tests for the columns that the text table leaves out because every
//! shown row lacks their field.

use std::process::ExitCode;

use clap::CommandFactory as _;
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::Value;

use super::{Tool, built_with, finish_output, finish_with};
use crate::help::Guide;
use crate::{Document, Listing, Row};

/// A row with a state that only some books have, a long `series` column
/// that the row leaves out when it has none, and a problem field.
#[derive(Serialize, JsonSchema)]
struct Book {
    /// The book's title.
    title: String,
    /// Who holds the book, or nothing when it is on the shelf.
    state: Option<&'static str>,
    /// The series that the book belongs to.
    #[serde(skip_serializing_if = "Option::is_none")]
    series: Option<&'static str>,
    /// What is wrong with the book, or nothing.
    problem: Option<&'static str>,
}

impl Row for Book {
    const COLUMNS: &'static [&'static str] = &["title", "state", "problem"];
    const LONG_COLUMNS: &'static [&'static str] = &["series"];
    const PROBLEM: Option<&'static str> = Some("problem");
}

const BOOKS: Document<Vec<Book>> = Document::new("shelf", "books", 1, "The test rows.");
const BOOK_LISTING: Document<Listing<Book>> =
    Document::new("shelf", "book-listing", 1, "The test rows.");

/// Returns a book on the shelf titled `title`, with no series and no
/// problem.
fn book(title: &str) -> Book {
    Book {
        title: title.to_owned(),
        state: None,
        series: None,
        problem: None,
    }
}

#[test]
fn text_leaves_out_a_column_that_is_null_in_every_row_and_the_problem_column_when_no_row_reports_one()
 {
    let finished = finish_with(&[], BOOKS, vec![book("Emma"), book("Persuasion")]);

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(finished.out, "TITLE\nEmma\nPersuasion\n");
}

#[test]
fn text_keeps_a_column_that_one_row_fills_and_prints_a_dash_in_the_others() {
    let lent = Book {
        state: Some("lent"),
        ..book("Persuasion")
    };

    let finished = finish_with(&[], BOOKS, vec![book("Emma"), lent]);

    assert_eq!(
        finished.out,
        "TITLE       STATE\nEmma        -\nPersuasion  lent\n"
    );
}

#[test]
fn text_keeps_the_problem_column_when_a_row_reports_a_problem() {
    let torn = Book {
        problem: Some("torn"),
        ..book("Emma")
    };

    let finished = finish_with(&[], BOOKS, vec![torn, book("Persuasion")]);

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert_eq!(
        finished.out,
        "TITLE       PROBLEM\nEmma        torn\nPersuasion  -\n"
    );
}

#[test]
fn long_leaves_out_a_long_column_that_every_row_is_missing() {
    let finished = finish_with(&["--long"], BOOKS, vec![book("Emma")]);

    assert_eq!(finished.out, "TITLE\nEmma\n");
}

#[test]
fn long_keeps_a_long_column_that_one_row_fills() {
    let sequel = Book {
        series: Some("Barsetshire"),
        ..book("Barchester Towers")
    };

    let finished = finish_with(&["--long"], BOOKS, vec![book("Emma"), sequel]);

    assert_eq!(
        finished.out,
        "TITLE              SERIES\nEmma               -\nBarchester Towers  Barsetshire\n"
    );
}

#[test]
fn fields_prints_a_named_column_that_is_null_in_every_row() {
    let finished = finish_with(&["--fields", "title,state"], BOOKS, vec![book("Emma")]);

    assert_eq!(finished.out, "TITLE  STATE\nEmma   -\n");
}

#[test]
fn json_keeps_a_column_that_is_null_in_every_row() {
    let finished = finish_with(&["--json"], BOOKS, vec![book("Emma")]);

    let document: Value = serde_json::from_str(&finished.out).unwrap();
    assert_eq!(document["data"][0]["state"], Value::Null);
    assert_eq!(document["data"][0]["problem"], Value::Null);
}

#[test]
fn a_listing_cut_by_the_budget_leaves_out_a_column_that_only_a_row_left_unshown_fills() {
    let mut rows: Vec<Book> = (0..40).map(|n| book(&"a".repeat(n + 60))).collect();
    rows.push(Book {
        state: Some("lent"),
        ..book("last")
    });
    let args = ["tool", "list"];
    let guide = Guide::new().output_budget(1024, "the tests print little");
    let output = built_with(Tool::command(), &guide, &args);

    let finished = finish_output(output, BOOK_LISTING, Listing::new(rows, 41));

    assert!(finished.out.starts_with("TITLE\na"), "{}", finished.out);
    assert!(!finished.out.contains("lent"), "{}", finished.out);
    assert!(finished.out.len() <= 1024, "{}", finished.out.len());
    assert!(finished.err.starts_with("Showing "), "{}", finished.err);
}
