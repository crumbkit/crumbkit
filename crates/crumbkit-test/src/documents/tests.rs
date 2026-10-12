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
fn a_document_named_after_its_leaf_s_path_passes() {
    const GUIDE: Guide = Guide::new().leaves(&[
        Leaf::new(&["books", "list"]).document(BOOKS_LIST),
        Leaf::new(&["commands", "list"]).document(COMMANDS_LIST),
    ]);

    check_document_ids(&Command::new("shelf"), &GUIDE, &[]);
}

#[test]
fn a_document_named_after_another_path_fails_and_names_the_expected_name() {
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "list"]).document(BOOK_LIST)]);

    let message =
        panic_message(|| check_document_ids(&Command::new("shelf"), &GUIDE, &[])).unwrap();

    assert_eq!(
        message,
        "the command tree breaks crumbkit's document id rules:\n\
         shelf books list: the command prints shelf/book-list/v1, and the name of the document that a command prints is the command's path below the tool joined with dashes, `books-list`"
    );
}

#[test]
fn a_cargo_subcommand_names_its_documents_after_the_path_below_its_own_name() {
    const GUIDE: Guide = Guide::new().leaves(&[
        Leaf::new(&["crumbkit", "weigh"]).document(WEIGH),
        Leaf::new(&["crumbkit", "commands", "list"]).document(COMMANDS_LIST),
    ]);

    check_document_ids(&Command::new("cargo"), &GUIDE, &[]);
    check_mounts(&Command::new("cargo"), &GUIDE);
}

#[test]
fn the_document_id_check_leaves_crumbkit_s_fixed_commands_to_the_mount_check() {
    const GUIDE: Guide =
        Guide::new().leaves(&[Leaf::new(&["meta", "schemas"]).document(SCHEMAS_LIST)]);

    check_document_ids(&Command::new("shelf"), &GUIDE, &[]);
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

#[test]
fn a_document_that_an_opt_out_names_may_have_any_name() {
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "list"]).document(BOOK_LIST)]);
    const ALLOW: &[Allow] = &[Allow::new(
        Rule::DocumentName,
        "books",
        "Scripts read the document under its name from before the rename.",
    )];

    check_document_ids(&shelf(), &GUIDE, ALLOW);
}

#[test]
fn an_opt_out_from_the_document_name_rule_that_excuses_no_problem_fails() {
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "list"]).document(BOOKS_LIST)]);
    const ALLOW: &[Allow] = &[Allow::new(Rule::DocumentName, "books list", "Old scripts.")];

    let message = panic_message(|| check_document_ids(&shelf(), &GUIDE, ALLOW)).unwrap();

    assert_eq!(
        message,
        "the command tree breaks crumbkit's document id rules:\n\
         the opt-out from `documents/document-name` for `shelf books list` excuses no problem, so it is stale"
    );
}

/// The document of `books list` without crumbkit's envelope, as scripts
/// read it before the tool adopted crumbkit.
const UNWRAPPED: Document<Vec<Book>> =
    Document::new("shelf", "books-list", 2, "One page of the books.").without_envelope();

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
