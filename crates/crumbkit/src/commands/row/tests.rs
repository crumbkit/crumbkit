//! Tests for the row that `commands list` prints for one command.

use clap::{Args as _, Command};
use serde_json::{Value, json};

use super::CommandRow;
use crate::help::{Effects, Guide, Leaf};

/// Returns the JSON of the row of `shelf books remove` with `guide`.
fn row(guide: &Guide) -> Value {
    let command = Command::new("remove").about("Remove books");
    let row = CommandRow::new("shelf", &["books", "remove"], &command, None, guide);
    serde_json::to_value(row).unwrap()
}

#[test]
fn a_row_holds_the_effects_that_the_leaf_declares() {
    const GUIDE: Guide = Guide::new()
        .leaves(&[Leaf::new(&["books", "remove"]).effects(Effects::destructive().idempotent())]);

    assert_eq!(
        row(&GUIDE)["effects"],
        json!({"read_only": false, "destructive": true, "idempotent": true, "open_world": false})
    );
}

#[test]
fn a_row_holds_null_effects_when_no_leaf_declares_them() {
    assert_eq!(row(&Guide::new())["effects"], Value::Null);
}

#[test]
fn a_row_holds_the_writes_of_a_replacing_leaf_and_whether_the_command_takes_expect() {
    const GUIDE: Guide = Guide::new()
        .leaves(&[Leaf::new(&["books", "remove"]).effects(Effects::replacing().all_or_none())]);
    let command = crate::Expect::augment_args(Command::new("remove"));
    let with_expect = CommandRow::new("shelf", &["books", "remove"], &command, None, &GUIDE);

    assert_eq!(
        serde_json::to_value(with_expect).unwrap()["writes"],
        json!({"replaces": true, "expect": true, "items": "all_or_none"})
    );
    assert_eq!(
        row(&GUIDE)["writes"],
        json!({"replaces": true, "expect": false, "items": "all_or_none"})
    );
}

#[test]
fn a_row_holds_null_writes_for_a_read_only_leaf_or_no_leaf() {
    const GUIDE: Guide =
        Guide::new().leaves(&[Leaf::new(&["books", "remove"]).effects(Effects::read_only())]);

    assert_eq!(row(&GUIDE)["writes"], Value::Null);
    assert_eq!(row(&Guide::new())["writes"], Value::Null);
}

/// A row type for the leaves of the form tests.
#[derive(serde::Serialize, schemars::JsonSchema)]
struct Book {
    /// The book's id.
    id: u32,
}

impl crate::Row for Book {
    const COLUMNS: &'static [&'static str] = &["id"];
}

#[test]
fn a_row_holds_the_form_document_when_the_leaf_declares_a_document() {
    const BOOKS: crate::Document<Vec<Book>> =
        crate::Document::new("shelf", "books-remove", 1, "The removed books.");
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "remove"]).document(BOOKS)]);

    assert_eq!(row(&GUIDE)["form"], json!("document"));
}

#[test]
fn a_row_holds_the_form_stream_when_the_leaf_declares_a_stream() {
    const BOOKS: crate::Document<crate::Stream<Book>> =
        crate::Document::new("shelf", "books-remove", 1, "Each removed book.");
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "remove"]).stream(BOOKS)]);

    assert_eq!(row(&GUIDE)["form"], json!("stream"));
}

#[test]
fn a_row_holds_the_form_bytes_when_the_leaf_declares_a_bytes_document() {
    const FILE: crate::Document<crate::Bytes> =
        crate::Document::new("shelf", "books-file", 1, "The removed book's file.");
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "remove"]).document(FILE)]);

    assert_eq!(row(&GUIDE)["form"], json!("bytes"));
}

#[test]
fn a_row_holds_a_null_form_when_no_leaf_declares_a_document() {
    assert_eq!(row(&Guide::new())["form"], Value::Null);
}

/// A removed book's file, which a headed document prints under a header.
#[derive(serde::Serialize, schemars::JsonSchema)]
struct BookFile {
    /// The book's id.
    id: u32,
    /// The file's bytes.
    #[serde(flatten)]
    content: crate::Bytes,
}

impl crate::Row for BookFile {
    const COLUMNS: &'static [&'static str] = &["id"];
}

impl crate::Headed for BookFile {
    fn bytes(&self) -> Option<&crate::Bytes> {
        Some(&self.content)
    }
}

#[test]
fn a_headed_row_has_the_headed_form_and_lists_its_json_fields_with_text_and_base64() {
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "remove"]).document(
        crate::Document::<crate::Batch<BookFile, crate::Error>>::new(
            "shelf",
            "books-remove",
            1,
            "The removed books' files.",
        )
        .headed(),
    )]);

    let row = row(&GUIDE);

    assert_eq!(row["form"], "headed");
    assert_eq!(
        row["columns"],
        json!([
            {"name": "id", "long": false, "description": "The book's id."},
            {"name": "text", "long": false, "description": "The bytes, which are valid UTF-8, as text."},
            {"name": "base64", "long": false, "description": "The bytes, which are not valid UTF-8, in standard base64 with padding (RFC 4648)."},
        ])
    );
}

/// A row that serializes as an array of its values, so it has no columns.
#[derive(serde::Serialize, schemars::JsonSchema)]
struct Answer(u32, u32);

impl crate::Row for Answer {
    const COLUMNS: &'static [&'static str] = &[];
    const WHOLE: bool = true;
}

#[test]
fn a_row_holds_the_form_json_rows_when_the_leaf_s_whole_rows_have_no_columns() {
    const ANSWERS: crate::Document<Vec<Answer>> =
        crate::Document::new("shelf", "books-query", 1, "The answers.");
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "remove"]).document(ANSWERS)]);

    let row = row(&GUIDE);

    assert_eq!(row["form"], json!("json-rows"));
    assert_eq!(row["columns"], json!([]));
}

/// A row that a tool's readers expect whole, which still names a column.
#[derive(serde::Serialize, schemars::JsonSchema)]
struct WholeBook {
    /// The book's id.
    id: u32,
}

impl crate::Row for WholeBook {
    const COLUMNS: &'static [&'static str] = &["id"];
    const WHOLE: bool = true;
}

#[test]
fn a_row_holds_the_form_document_when_the_leaf_s_whole_rows_have_columns() {
    const BOOKS: crate::Document<Vec<WholeBook>> =
        crate::Document::new("shelf", "books-remove", 1, "The removed books.");
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "remove"]).document(BOOKS)]);

    assert_eq!(row(&GUIDE)["form"], json!("document"));
}
