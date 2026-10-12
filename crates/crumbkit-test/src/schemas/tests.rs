//! Tests for the schema golden check.

use std::fs;

use clap::Command;
use crumbkit::help::{Guide, Leaf};
use crumbkit::schemas::{SCHEMAS_LIST, document_schema};
use crumbkit::{Document, Listing, Row};
use schemars::JsonSchema;
use serde::Serialize;
use snapbox::Assert;
use snapbox::assert::Action;

use super::check;
use crate::temp::TempDir;
use crate::testing::panic_message;

/// A row of the test tool.
#[derive(Serialize, JsonSchema)]
struct Book {
    /// The book's id.
    id: u32,
}

impl Row for Book {
    const COLUMNS: &'static [&'static str] = &["id"];
}

/// Another row, whose schema differs from [`Book`]'s.
#[derive(Serialize, JsonSchema)]
struct Author {
    /// The author's name.
    name: String,
}

impl Row for Author {
    const COLUMNS: &'static [&'static str] = &["name"];
}

const BOOKS_LIST: Document<Listing<Book>> =
    Document::new("shelf", "books-list", 1, "One page of the books.");

/// A guide with a tool document and one of crumbkit's.
const GUIDE: Guide = Guide::new().leaves(&[
    Leaf::new(&["books", "list"]).document(BOOKS_LIST),
    Leaf::new(&["schemas", "list"]).document(SCHEMAS_LIST),
]);

/// A tool with the leaves of [`GUIDE`].
fn shelf() -> Command {
    Command::new("shelf")
        .subcommand(Command::new("books").subcommand(Command::new("list")))
        .subcommand(Command::new("schemas").subcommand(Command::new("list")))
}

/// Runs the check on `shelf()` with `guide` against `dir` with `action`.
fn run(guide: &Guide, dir: &TempDir, action: Action) {
    check(&shelf(), guide, dir.path(), &Assert::new().action(action));
}

#[test]
fn overwrite_writes_one_file_for_each_tool_document_and_deletes_a_stale_one() {
    let dir = TempDir::new("schemas-overwrite").unwrap();
    dir.write("shelf-books-gone-v1.json", "{}\n");
    dir.write("notes.md", "Only .json files are schema golden files.\n");

    run(&GUIDE, &dir, Action::Overwrite);

    assert_eq!(dir.names(), ["notes.md", "shelf-books-list-v1.json"]);
    let written = fs::read_to_string(dir.path().join("shelf-books-list-v1.json")).unwrap();
    assert_eq!(
        written,
        serde_json::to_string_pretty(&document_schema(BOOKS_LIST)).unwrap() + "\n"
    );
}

#[test]
fn files_that_match_every_tool_document_pass() {
    let dir = TempDir::new("schemas-pass").unwrap();
    run(&GUIDE, &dir, Action::Overwrite);

    run(&GUIDE, &dir, Action::Verify);
}

#[test]
fn a_missing_a_differing_and_a_stale_file_each_fail() {
    let dir = TempDir::new("schemas-fail").unwrap();
    dir.write("shelf-books-gone-v1.json", "{}\n");

    let missing = panic_message(|| run(&GUIDE, &dir, Action::Verify)).unwrap();
    dir.write("shelf-books-list-v1.json", "{}\n");
    let differing = panic_message(|| run(&GUIDE, &dir, Action::Verify)).unwrap();

    assert!(
        missing.starts_with(
            "the command tree breaks crumbkit's golden schema rules:\n\
             shelf/books-list/v1: the golden file `"
        ),
        "{missing}"
    );
    assert!(
        missing.contains("shelf-books-list-v1.json` is missing"),
        "{missing}"
    );
    assert!(
        missing.contains("shelf-books-gone-v1.json` belongs to no document"),
        "{missing}"
    );
    assert!(
        differing.contains("shelf/books-list/v1: the schema differs from the golden file"),
        "{differing}"
    );
}

#[test]
fn two_leaves_that_declare_one_id_with_different_schemas_fail() {
    const AUTHORS: Document<Vec<Author>> =
        Document::new("shelf", "books-list", 1, "One page of the books.");
    const TWICE: Guide = Guide::new().leaves(&[
        Leaf::new(&["books", "list"]).document(BOOKS_LIST),
        Leaf::new(&["schemas", "list"]).document(AUTHORS),
    ]);
    let dir = TempDir::new("schemas-twice").unwrap();

    let message = panic_message(|| run(&TWICE, &dir, Action::Overwrite)).unwrap();

    assert_eq!(
        message,
        "the command tree breaks crumbkit's golden schema rules:\n\
         shelf/books-list/v1: `shelf books list` and `shelf schemas list` print the document with different schemas, so one of them needs a document of its own"
    );
}

#[test]
fn two_ids_that_share_a_golden_file_name_fail() {
    const DASHED: Document<Listing<Book>> =
        Document::new("shelf-books", "list", 1, "One page of the books.");
    const SHARED: Guide = Guide::new().leaves(&[
        Leaf::new(&["books", "list"]).document(BOOKS_LIST),
        Leaf::new(&["schemas", "list"]).document(DASHED),
    ]);
    let dir = TempDir::new("schemas-shared").unwrap();

    let message = panic_message(|| run(&SHARED, &dir, Action::Overwrite)).unwrap();

    assert_eq!(
        message,
        "the command tree breaks crumbkit's golden schema rules:\n\
         shelf-books/list/v1: the golden file name `shelf-books-list-v1.json` is also the name for the document shelf/books-list/v1, so one of the two needs another id"
    );
}
