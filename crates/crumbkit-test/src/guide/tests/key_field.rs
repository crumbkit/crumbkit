//! Tests for the guide check of the field under which a batch's per-key
//! over-budget failures hold their key.

use crumbkit::help::Guide;
use crumbkit::{Batch, Document, Error, Row};
use schemars::JsonSchema;
use serde::Serialize;

use super::{check_guide, leaf, panic_message, shelf};

/// A book of `shelf books show`.
#[derive(Serialize, JsonSchema)]
struct Book {
    /// The book's id.
    id: u32,
}

impl Row for Book {
    const COLUMNS: &'static [&'static str] = &["id"];
}

/// The document of `shelf books show`, which answers each key left out.
const BOOKS: Document<Batch<Book, Error>> =
    Document::new("shelf", "books-show", 1, "The books.").left_out_per_key();

#[test]
fn a_key_field_that_follows_the_field_name_rule_passes() {
    const GUIDE: Guide = Guide::new().leaves(&[
        leaf(&["books", "list"]),
        leaf(&["books", "show"]).document(BOOKS.key_field("book_id")),
        leaf(&["authors", "list"]),
    ]);

    check_guide(&shelf(), &GUIDE);
}

#[test]
fn a_key_field_that_the_guide_lists_among_its_field_names_passes() {
    const GUIDE: Guide = Guide::new().field_names(&["bookId"]).leaves(&[
        leaf(&["books", "list"]),
        leaf(&["books", "show"]).document(BOOKS.key_field("bookId")),
        leaf(&["authors", "list"]),
    ]);

    check_guide(&shelf(), &GUIDE);
}

#[test]
fn a_key_field_that_breaks_the_field_name_rule_fails_the_check() {
    const GUIDE: Guide = Guide::new().leaves(&[
        leaf(&["books", "list"]),
        leaf(&["books", "show"]).document(BOOKS.key_field("bookId")),
        leaf(&["authors", "list"]),
    ]);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE)).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         shelf books show: the key field \"bookId\" breaks crumbkit's field name rule and the guide's field_names do not list it, so each over-budget failure would leave it out"
    );
}

#[test]
fn a_key_field_named_budget_fails_the_check() {
    const GUIDE: Guide = Guide::new().leaves(&[
        leaf(&["books", "list"]),
        leaf(&["books", "show"]).document(BOOKS.key_field("budget")),
        leaf(&["authors", "list"]),
    ]);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE)).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         shelf books show: the key field is named \"budget\", which the over-budget failure already uses for the bytes of the output budget; choose another name"
    );
}
