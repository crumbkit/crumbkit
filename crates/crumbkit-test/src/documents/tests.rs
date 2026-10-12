//! Tests for the document id check and the mount check.

use clap::Command;
use crumbkit::commands::COMMANDS_LIST;
use crumbkit::form::{FailureForm, PreparedFailure};
use crumbkit::help::{Guide, Leaf};
use crumbkit::schemas::{SCHEMAS_LIST, SCHEMAS_SHOW};
use crumbkit::{Document, Row};
use schemars::{JsonSchema, Schema, json_schema};
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

/// A failure form whose document has the id that `document` returns.
struct ErrorForm(fn() -> Document<()>);

impl FailureForm for ErrorForm {
    fn document(&self) -> Document<()> {
        (self.0)()
    }

    fn schema(&self) -> Schema {
        json_schema!({ "type": "object" })
    }

    fn json(&self, failure: &PreparedFailure<'_>) -> serde_json::Value {
        serde_json::json!({ "error": failure.message() })
    }
}

/// The failure form of a tool whose callers read `shelf/error/v2`.
static SHELF_FORM: ErrorForm =
    ErrorForm(|| Document::new("shelf", "error", 2, "The document of a failed command."));

/// The opt-out that keeps [`SHELF_FORM`].
const FORM_ALLOW: &[Allow] = &[Allow::new(
    Rule::FailureForm,
    "",
    "Scripts read the error document that shelf printed before it adopted crumbkit.",
)];

#[test]
fn a_guide_with_a_failure_form_and_without_an_opt_out_fails() {
    const GUIDE: Guide = Guide::new().failure_form(&SHELF_FORM);

    let message = panic_message(|| check_document_ids(&shelf(), &GUIDE, &[])).unwrap();

    assert_eq!(
        message,
        "the command tree breaks crumbkit's document id rules:\n\
         shelf: the tool prints its failures as shelf/error/v2 in place of crumbkit/failure/v1, which needs an opt-out from `documents/failure-form` with a reason"
    );
}

#[test]
fn a_guide_with_a_failure_form_that_an_opt_out_at_the_root_names_passes() {
    const GUIDE: Guide = Guide::new().failure_form(&SHELF_FORM);

    check_document_ids(&shelf(), &GUIDE, FORM_ALLOW);
}

#[test]
fn an_opt_out_from_the_failure_form_rule_without_a_failure_form_is_stale() {
    let message =
        panic_message(|| check_document_ids(&shelf(), &Guide::new(), FORM_ALLOW)).unwrap();

    assert_eq!(
        message,
        "the command tree breaks crumbkit's document id rules:\n\
         the opt-out from `documents/failure-form` for `shelf` excuses no problem, so it is stale"
    );
}

#[test]
fn a_failure_form_document_in_the_component_crumbkit_fails_even_with_an_opt_out() {
    static FORM: ErrorForm =
        ErrorForm(|| Document::new("crumbkit", "failure", 2, "A failure document."));
    const GUIDE: Guide = Guide::new().failure_form(&FORM);

    let message = panic_message(|| check_document_ids(&shelf(), &GUIDE, FORM_ALLOW)).unwrap();

    assert_eq!(
        message,
        "the command tree breaks crumbkit's document id rules:\n\
         shelf: the tool's failure form prints crumbkit/failure/v2, and only crumbkit names documents in the component `crumbkit`, so the form's document needs the tool's own component"
    );
}
