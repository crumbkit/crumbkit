//! Tests for the guide check of the field that `Row::CHILDREN` names
//! (ADR-0120).

use crumbkit::help::Guide;
use crumbkit::{Batch, Bytes, Document, Error, Headed, Row};
use schemars::JsonSchema;
use serde::Serialize;

use super::{check_guide, leaf, panic_message, shelf};

/// A book whose chapters print under it.
#[derive(Serialize, JsonSchema)]
struct Book {
    /// The book's id.
    id: u32,
    /// The book's title.
    title: String,
    /// The titles of the book's chapters.
    chapters: Vec<String>,
}

impl Row for Book {
    const COLUMNS: &'static [&'static str] = &["id"];
    const LONG_COLUMNS: &'static [&'static str] = &["title"];
    const CHILDREN: Option<&'static str> = Some("chapters");
}

/// A book whose children field is not one of its fields.
#[derive(Serialize, JsonSchema)]
struct MisspeltBook {
    /// The book's id.
    id: u32,
    /// The titles of the book's chapters.
    chapters: Vec<String>,
}

impl Row for MisspeltBook {
    const COLUMNS: &'static [&'static str] = &["id"];
    const CHILDREN: Option<&'static str> = Some("chapter");
}

/// A book whose children field has no doc comment.
#[derive(Serialize, JsonSchema)]
struct UndocumentedBook {
    /// The book's id.
    id: u32,
    chapters: Vec<String>,
}

impl Row for UndocumentedBook {
    const COLUMNS: &'static [&'static str] = &["id"];
    const CHILDREN: Option<&'static str> = Some("chapters");
}

/// A book whose children field is also one of its columns.
#[derive(Serialize, JsonSchema)]
struct ColumnBook {
    /// The book's id.
    id: u32,
    /// The titles of the book's chapters.
    chapters: Vec<String>,
}

impl Row for ColumnBook {
    const COLUMNS: &'static [&'static str] = &["id", "chapters"];
    const CHILDREN: Option<&'static str> = Some("chapters");
}

/// A book whose children field is also one of its long columns.
#[derive(Serialize, JsonSchema)]
struct LongColumnBook {
    /// The book's id.
    id: u32,
    /// The titles of the book's chapters.
    chapters: Vec<String>,
}

impl Row for LongColumnBook {
    const COLUMNS: &'static [&'static str] = &["id"];
    const LONG_COLUMNS: &'static [&'static str] = &["chapters"];
    const CHILDREN: Option<&'static str> = Some("chapters");
}

/// A book's file, which a headed batch prints under a header, and which
/// also names a children field.
#[derive(Serialize, JsonSchema)]
struct ChapteredFile {
    /// The book's id.
    id: u32,
    /// The titles of the book's chapters.
    chapters: Vec<String>,
    /// The file's bytes.
    #[serde(flatten)]
    content: Bytes,
}

impl Row for ChapteredFile {
    const COLUMNS: &'static [&'static str] = &["id"];
    const CHILDREN: Option<&'static str> = Some("chapters");
}

impl Headed for ChapteredFile {
    fn bytes(&self) -> Option<&Bytes> {
        Some(&self.content)
    }
}

/// Returns the problem that `check_guide` reports for a guide whose
/// `books list` leaf has `document`, without the heading line.
fn problem<T: crumbkit::Rows>(document: Document<T>) -> String {
    let guide = Guide::new().leaves(Box::leak(Box::new([
        leaf(&["books", "list"]).document(document),
        leaf(&["books", "show"]),
        leaf(&["authors", "list"]),
    ])));
    let message = panic_message(|| check_guide(&shelf(), &guide, &[])).unwrap();
    message
        .strip_prefix("the command tree breaks crumbkit's guide rules:\n")
        .unwrap_or(&message)
        .to_owned()
}

#[test]
fn a_row_type_whose_children_field_is_a_documented_field_outside_the_columns_passes() {
    const GUIDE: Guide = Guide::new().leaves(&[
        leaf(&["books", "list"]).document(Document::<Vec<Book>>::new(
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
fn check_guide_fails_on_a_children_field_that_is_not_a_field_of_the_row_type() {
    let document = Document::<Vec<MisspeltBook>>::new("shelf", "books-list", 1, "The test rows.");

    assert_eq!(
        problem(document),
        "shelf books list: the children field `chapter` is not a field of the row type, so the JSON holds no children for the lines that the text prints under each row; name a field of the row type in `Row::CHILDREN`"
    );
}

#[test]
fn check_guide_fails_on_a_children_field_without_a_doc_comment() {
    let document =
        Document::<Vec<UndocumentedBook>>::new("shelf", "books-list", 1, "The test rows.");

    assert_eq!(
        problem(document),
        "shelf books list: the children field `chapters` has no description, because the field `chapters` of the row type has no doc comment; write a doc comment on the field"
    );
}

#[test]
fn check_guide_fails_on_a_children_field_that_is_also_a_column() {
    let document = Document::<Vec<ColumnBook>>::new("shelf", "books-list", 1, "The test rows.");

    assert_eq!(
        problem(document),
        "shelf books list: the children field `chapters` is also one of the row type's COLUMNS, so the table prints the children as a cell of JSON and again as lines under the row; remove `chapters` from COLUMNS"
    );
}

#[test]
fn check_guide_fails_on_a_children_field_that_is_also_a_long_column() {
    let document = Document::<Vec<LongColumnBook>>::new("shelf", "books-list", 1, "The test rows.");

    assert_eq!(
        problem(document),
        "shelf books list: the children field `chapters` is also one of the row type's LONG_COLUMNS, so the table prints the children as a cell of JSON and again as lines under the row; remove `chapters` from LONG_COLUMNS"
    );
}

#[test]
fn check_guide_fails_on_a_headed_leaf_whose_row_type_names_a_children_field() {
    let document =
        Document::<Batch<ChapteredFile, Error>>::new("shelf", "books-list", 1, "The test rows.")
            .headed();

    assert_eq!(
        problem(document),
        "shelf books list: the row type declares CHILDREN and the document is headed, so its text prints each row's bytes and never the child lines; remove CHILDREN or the headed form"
    );
}
