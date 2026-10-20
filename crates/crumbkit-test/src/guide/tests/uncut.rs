//! Tests for the guide check of the columns that `Row::UNCUT_COLUMNS`
//! names (ADR-0121).

use crumbkit::help::Guide;
use crumbkit::{Document, Row};
use schemars::JsonSchema;
use serde::Serialize;

use super::{check_guide, leaf, panic_message, shelf};

/// A book whose id and long ISBN are never cut at a terminal.
#[derive(Serialize, JsonSchema)]
struct KeptBook {
    /// The book's id.
    id: u32,
    /// The book's title.
    title: String,
    /// The book's ISBN.
    isbn: String,
}

impl Row for KeptBook {
    const COLUMNS: &'static [&'static str] = &["id", "title"];
    const LONG_COLUMNS: &'static [&'static str] = &["isbn"];
    const UNCUT_COLUMNS: &'static [&'static str] = &["id", "isbn"];
}

/// A book whose uncut column is a misspelt name.
#[derive(Serialize, JsonSchema)]
struct MisspeltBook {
    /// The book's id.
    id: u32,
    /// The book's title.
    title: String,
}

impl Row for MisspeltBook {
    const COLUMNS: &'static [&'static str] = &["id", "title"];
    const UNCUT_COLUMNS: &'static [&'static str] = &["ids"];
}

#[test]
fn uncut_columns_that_are_compact_or_long_columns_pass() {
    const GUIDE: Guide = Guide::new().leaves(&[
        leaf(&["books", "list"]).document(Document::<Vec<KeptBook>>::new(
            "shelf",
            "books-list",
            1,
            "The test rows.",
        )),
        leaf(&["books", "show"]),
        leaf(&["authors", "list"]),
    ]);

    check_guide(&shelf(), &GUIDE, &[]);
}

#[test]
fn check_guide_fails_on_an_uncut_column_that_is_not_a_column_of_the_row_type() {
    const GUIDE: Guide = Guide::new().leaves(&[
        leaf(&["books", "list"]).document(Document::<Vec<MisspeltBook>>::new(
            "shelf",
            "books-list",
            1,
            "The test rows.",
        )),
        leaf(&["books", "show"]),
        leaf(&["authors", "list"]),
    ]);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE, &[])).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         shelf books list: the row type's UNCUT_COLUMNS names `ids`, which is in neither COLUMNS nor LONG_COLUMNS, so the name keeps no column of the table whole; name a column from COLUMNS or LONG_COLUMNS, or remove the name"
    );
}
