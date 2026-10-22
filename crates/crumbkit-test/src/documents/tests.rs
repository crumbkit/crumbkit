//! Tests for the document id check and the mount check.

use clap::Command;
use crumbkit::commands::COMMANDS_LIST;
use crumbkit::help::{Guide, Leaf};
use crumbkit::schemas::{SCHEMAS_LIST, SCHEMAS_SHOW};
use crumbkit::{Document, Row};
use schemars::JsonSchema;
use serde::Serialize;

use super::{MOUNTS, check_document_ids, check_mounts};
use crate::testing::panic_message;
use crate::{Allow, Rule};
use crumbkit::Envelope;

/// A row of the test tools.
#[derive(Serialize, JsonSchema)]
struct Book {
    /// The book's id.
    id: u32,
}

impl Row for Book {
    const COLUMNS: &'static [&'static str] = &["id"];
}

const BOOKS_LIST: Document<Vec<Book>> =
    Document::new("shelf", "books-list", 1, "One page of the books.");
const BOOK_LIST: Document<Vec<Book>> =
    Document::new("shelf", "book-list", 1, "One page of the books.");
const WEIGH: Document<Vec<Book>> = Document::new("crumbkit", "weigh", 1, "The weights.");

#[test]
fn a_cargo_subcommand_mounts_crumbkit_s_commands_below_its_own_name() {
    const GUIDE: Guide = Guide::new().leaves(&[
        Leaf::new(&["crumbkit", "weigh"]).document(WEIGH),
        Leaf::new(&["crumbkit", "commands", "list"]).document(COMMANDS_LIST),
    ]);

    check_mounts(&Command::new("cargo"), &GUIDE);
}

#[test]
fn crumbkit_s_commands_at_their_fixed_paths_pass() {
    const GUIDE: Guide = Guide::new().leaves(&[
        Leaf::new(&["commands", "list"]).document(COMMANDS_LIST),
        Leaf::new(&["schemas", "list"]).document(SCHEMAS_LIST),
        Leaf::new(&["schemas", "show"]).document(SCHEMAS_SHOW),
        Leaf::new(&["books", "list"]).document(BOOKS_LIST),
    ]);

    check_mounts(&Command::new("shelf"), &GUIDE);
}

#[test]
fn a_document_named_after_another_path_is_a_declaration_and_passes() {
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "list"]).document(BOOK_LIST)]);

    check_document_ids(&Command::new("shelf"), &GUIDE, &[]);
}

#[test]
fn a_crumbkit_command_at_another_path_fails_and_names_its_fixed_path() {
    const GUIDE: Guide =
        Guide::new().leaves(&[Leaf::new(&["meta", "schemas"]).document(SCHEMAS_LIST)]);

    let message = panic_message(|| check_mounts(&Command::new("shelf"), &GUIDE)).unwrap();

    assert_eq!(
        message,
        "the command tree breaks crumbkit's mount rules:\n\
         shelf meta schemas: the command prints crumbkit/schemas-list/v1, which crumbkit's command `schemas list` prints, so the tool mounts it at `shelf schemas list`"
    );
}

#[test]
fn each_fixed_path_names_its_document_by_the_document_id_rule() {
    for (name, path) in MOUNTS {
        assert_eq!(name, path.join("-"));
    }
}

/// A tool with the command `books list`.
fn shelf() -> Command {
    Command::new("shelf").subcommand(Command::new("books").subcommand(Command::new("list")))
}

/// The document of `books list` without crumbkit's envelope, as scripts
/// read it before the tool adopted crumbkit.
const UNWRAPPED: Document<Vec<Book>> =
    Document::new("shelf", "books-list", 2, "One page of the books.").envelope(Envelope::Without);

#[test]
fn a_document_without_the_envelope_and_without_an_opt_out_fails() {
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "list"]).document(UNWRAPPED)]);

    let message = panic_message(|| check_document_ids(&shelf(), &GUIDE, &[])).unwrap();

    assert_eq!(
        message,
        "the command tree breaks crumbkit's document id rules:\n\
         shelf books list: the command prints shelf/books-list/v2 without crumbkit's envelope, which needs an opt-out from `documents/envelope` with a reason"
    );
}

#[test]
fn a_document_without_the_envelope_that_an_opt_out_names_passes() {
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "list"]).document(UNWRAPPED)]);
    const ALLOW: &[Allow] = &[Allow::new(
        Rule::Envelope,
        "books list",
        "Scripts read the bare array that shelf printed before it adopted crumbkit.",
    )];

    check_document_ids(&shelf(), &GUIDE, ALLOW);
}

/// A book that the tool's readers expect whole, so `--json` prints every
/// field of it.
#[derive(Serialize, JsonSchema)]
struct WholeBook {
    /// The book's id.
    id: u32,
}

impl Row for WholeBook {
    const COLUMNS: &'static [&'static str] = &["id"];
    const WHOLE: bool = true;
}

const WHOLE_BOOKS: Document<Vec<WholeBook>> =
    Document::new("shelf", "books-list", 1, "One page of the books.");

#[test]
fn a_document_whose_rows_print_whole_without_an_opt_out_fails() {
    const GUIDE: Guide =
        Guide::new().leaves(&[Leaf::new(&["books", "list"]).document(WHOLE_BOOKS)]);

    let message = panic_message(|| check_document_ids(&shelf(), &GUIDE, &[])).unwrap();

    assert_eq!(
        message,
        "the command tree breaks crumbkit's document id rules:\n\
         shelf books list: the command prints the rows of shelf/books-list/v1 whole, because its row type sets `Row::WHOLE`, which needs an opt-out from `documents/rows-whole` with a reason"
    );
}

#[test]
fn a_document_whose_rows_print_whole_that_an_opt_out_names_passes() {
    const GUIDE: Guide =
        Guide::new().leaves(&[Leaf::new(&["books", "list"]).document(WHOLE_BOOKS)]);
    const ALLOW: &[Allow] = &[Allow::new(
        Rule::RowsWhole,
        "books list",
        "Scripts read every field of each book, as shelf printed it before it adopted crumbkit.",
    )];

    check_document_ids(&shelf(), &GUIDE, ALLOW);
}

#[test]
fn a_leaf_that_prints_no_result_passes_the_document_id_check_without_an_opt_out() {
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["serve"]).prints_no_result()]);

    check_document_ids(&Command::new("shelf"), &GUIDE, &[]);
}
