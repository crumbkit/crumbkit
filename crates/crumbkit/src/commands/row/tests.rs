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

/// A removed book whose text is its own line.
#[derive(serde::Serialize, schemars::JsonSchema)]
struct Removed {
    /// The book's id.
    id: u32,
}

impl crate::Row for Removed {
    const COLUMNS: &'static [&'static str] = &["id"];
    const LINE: Option<&'static str> = Some("removed <ID>");
}

#[test]
fn a_row_holds_the_text_line_form_of_its_leaf_s_rows() {
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "remove"]).document(
        crate::Document::<Vec<Removed>>::new("shelf", "books-remove", 1, "The removed books."),
    )]);

    assert_eq!(row(&GUIDE)["line"], json!("removed <ID>"));
}

#[test]
fn a_row_holds_a_null_line_when_its_leaf_s_rows_print_a_table() {
    const BOOKS: crate::Document<Vec<Book>> =
        crate::Document::new("shelf", "books-remove", 1, "The removed books.");
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "remove"]).document(BOOKS)]);

    assert_eq!(row(&GUIDE)["line"], Value::Null);
    assert!(row(&Guide::new()).as_object().unwrap().contains_key("line"));
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

/// A row that serializes as an array, prints whole, and has its own text
/// line.
#[derive(serde::Serialize, schemars::JsonSchema)]
struct Setting(u32, u32);

impl crate::Row for Setting {
    const COLUMNS: &'static [&'static str] = &[];
    const WHOLE: bool = true;
    const LINE: Option<&'static str> = Some("<KEY> = <VALUE>");
}

#[test]
fn a_row_holds_the_form_document_and_the_line_when_whole_rows_without_columns_declare_a_line() {
    const SETTINGS: crate::Document<Vec<Setting>> =
        crate::Document::new("shelf", "books-remove", 1, "The settings.");
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "remove"]).document(SETTINGS)]);

    let row = row(&GUIDE);

    assert_eq!(row["form"], json!("document"));
    assert_eq!(row["line"], json!("<KEY> = <VALUE>"));
}

/// A removed shelf whose books print under it.
#[derive(serde::Serialize, schemars::JsonSchema)]
struct Shelf {
    /// The shelf's id.
    id: u32,
    /// The titles of the books on the shelf.
    books: Vec<String>,
}

impl crate::Row for Shelf {
    const COLUMNS: &'static [&'static str] = &["id"];
    const CHILDREN: Option<&'static str> = Some("books");
}

#[test]
fn a_row_holds_the_children_field_of_its_leaf_s_rows_and_null_without_one() {
    const SHELVES: crate::Document<Vec<Shelf>> =
        crate::Document::new("shelf", "books-remove", 1, "The removed shelves.");
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "remove"]).document(SHELVES)]);

    assert_eq!(row(&GUIDE)["children"], json!("books"));
    assert_eq!(row(&Guide::new())["children"], Value::Null);
    assert!(
        row(&Guide::new())
            .as_object()
            .unwrap()
            .contains_key("children")
    );
}

/// A removed shelf whose books print under it, folded into one line when
/// the shelf held more than ten, which its help says.
#[derive(serde::Serialize, schemars::JsonSchema)]
struct FoldedShelf {
    /// The shelf's id.
    id: u32,
    /// The titles of the books on the shelf.
    books: Vec<String>,
}

impl crate::Row for FoldedShelf {
    const COLUMNS: &'static [&'static str] = &["id"];
    const CHILDREN: Option<&'static str> = Some("books");
    const CHILDREN_HELP: Option<&'static str> =
        Some("one line for each book, or one folded line for more than ten books.");
}

#[test]
fn a_row_holds_the_text_that_help_prints_on_the_children_line_and_null_without_children() {
    const SHELVES: crate::Document<Vec<Shelf>> =
        crate::Document::new("shelf", "books-remove", 1, "The removed shelves.");
    const FOLDED: crate::Document<Vec<FoldedShelf>> =
        crate::Document::new("shelf", "books-remove", 1, "The removed shelves.");
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "remove"]).document(SHELVES)]);
    const FOLDED_GUIDE: Guide =
        Guide::new().leaves(&[Leaf::new(&["books", "remove"]).document(FOLDED)]);

    assert_eq!(
        row(&GUIDE)["children_help"],
        json!("one indented line under each row for each of its books.")
    );
    assert_eq!(
        row(&FOLDED_GUIDE)["children_help"],
        json!("one line for each book, or one folded line for more than ten books.")
    );
    assert_eq!(row(&Guide::new())["children_help"], Value::Null);
    assert!(
        row(&Guide::new())
            .as_object()
            .unwrap()
            .contains_key("children_help")
    );
}

#[test]
fn help_prints_the_children_text_of_the_row_on_its_children_line() {
    const FOLDED: crate::Document<Vec<FoldedShelf>> =
        crate::Document::new("shelf", "books-remove", 1, "The removed shelves.");
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "remove"])
        .examples(&[crate::help::Example::new(&[], "Remove the books")])
        .document(FOLDED)]);
    let tool = Command::new("shelf")
        .subcommand(Command::new("books").subcommand(Command::new("remove").about("Remove books")));
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = crate::Sink::new(&mut out, &mut err);
    let args = ["shelf", "books", "remove", "-h"];
    let _ = crate::parse_to::<(), _, _, _, _>(&mut sink, tool, &GUIDE, args);

    let page = String::from_utf8(out).unwrap();
    let row = row(&GUIDE);
    let text = row["children_help"].as_str().unwrap();
    let line = crate::help::children_line(text);
    assert!(page.contains(&format!("\n{line}\n")), "{page}");
}
