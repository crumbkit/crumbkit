//! Tests for the customisations section of the opt-out report, which lists
//! the tool's declarations without a reason or a cost.

use clap::Command;
use crumbkit::codes::CLI_USAGE;
use crumbkit::form::{FailureForm, PreparedFailure};
use crumbkit::help::{Exit, Guide, Leaf, RefusalContext, Rename};
use crumbkit::schemas::SCHEMAS_LIST;
use crumbkit::{Batch, Call, Code, Document, JsonStyle, Prose};
use schemars::{Schema, json_schema};
use serde_json::{Value, json};
use snapbox::assert::Action;

use super::{read, run_guided};
use crate::opt_outs::customisations::customisations;
use crate::temp::TempDir;
use crumbkit::Envelope;

/// A failure form whose document is `shelf/error/v2`.
struct ErrorForm;

impl FailureForm for ErrorForm {
    fn document(&self) -> Document<()> {
        Document::new("shelf", "error", 2, "The document of a failed command.")
    }

    fn schema(&self) -> Schema {
        json_schema!({ "type": "object" })
    }

    fn json(&self, failure: &PreparedFailure<'_>) -> Value {
        json!({ "error": failure.message() })
    }
}

/// Returns the call that a refused `books show` command line offers instead.
fn version_call(_: &RefusalContext<'_>) -> Vec<Call> {
    vec![Call::new(&["version"])]
}

/// The tool's code for crumbkit's usage code.
const USAGE: Code = Code::new("shelf", "cli", "usage");

/// A guide that declares a failure form, a rename, an exit for the renamed
/// usage code, refusal alternatives for every command line and a leaf that passes on its program's exit code and offers
/// its own alternatives when its command line is refused.
const DECLARING: Guide = Guide::new()
    .failure_form(&ErrorForm)
    .codes(&[Rename::new(&CLI_USAGE, &USAGE)])
    .exits(&[Exit::new(64, "The command line was refused.").failures(&[&USAGE])])
    .refusal_alternatives(version_call)
    .leaves(&[Leaf::new(&["books", "show"])
        .passes_on_exit("the pager")
        .refusal_alternatives(version_call)]);

#[test]
fn the_report_lists_declarations_under_customisations_without_a_reason_or_a_cost() {
    let dir = TempDir::new("opt-outs-customisations").unwrap();

    run_guided(&DECLARING, &dir, Action::Overwrite);

    assert_eq!(
        read(&dir, "opt-outs.txt"),
        "shelf has no opt-outs from the conventions.\n\
         \n\
         shelf declares 6 customisations, which need no reason.\n\
         \n\
         failure-form: every run of shelf\n\
         \x20 Declares: Failures print as shelf/error/v2 in place of shelf/failure/v1.\n\
         \n\
         code-rename: every run of shelf\n\
         \x20 Declares: Runs print shelf::cli::usage in place of shelf::cli::usage.\n\
         \n\
         exit-code: every run of shelf\n\
         \x20 Declares: A failure with shelf::cli::usage exits with 64, where the default is 2.\n\
         \n\
         refusal-alternatives: every run of shelf\n\
         \x20 Declares: A refused command line for a command without its own function also offers the tool's own alternatives.\n\
         \n\
         passes-on-exit: shelf books show\n\
         \x20 Declares: The command exits with the exit code of the pager.\n\
         \n\
         refusal-alternatives: shelf books show\n\
         \x20 Declares: A refused command line for this command also offers the tool's own alternatives.\n"
    );
    let report: Value = serde_json::from_str(&read(&dir, "opt-outs.json")).unwrap();
    assert_eq!(report["data"], json!([]));
    assert_eq!(
        report["customisations"][4],
        json!({
            "declaration": "passes-on-exit",
            "applies_to": "shelf books show",
            "declares": "The command exits with the exit code of the pager.",
        })
    );
    assert_eq!(report["customisations"].as_array().unwrap().len(), 6);
}

#[test]
fn a_report_without_declarations_leaves_out_customisations() {
    let dir = TempDir::new("opt-outs-no-customisations").unwrap();

    run_guided(&Guide::new(), &dir, Action::Overwrite);

    let report: Value = serde_json::from_str(&read(&dir, "opt-outs.json")).unwrap();
    assert_eq!(report, json!({ "schema": "shelf/opt-outs/v1", "data": [] }));
}

#[test]
fn an_exit_that_lists_the_tool_s_own_code_is_no_customisation() {
    const MINE: Code = Code::new("shelf", "books", "not_found");
    const GUIDE: Guide = Guide::new().exits(&[Exit::new(3, "No such book.").failures(&[&MINE])]);
    let dir = TempDir::new("opt-outs-own-exit").unwrap();

    run_guided(&GUIDE, &dir, Action::Overwrite);

    assert_eq!(
        read(&dir, "opt-outs.txt"),
        "shelf has no opt-outs from the conventions.\n"
    );
}

const BOOK_SHOW: Document<Prose<String>> = Document::new("shelf", "book-show", 1, "One book.");
const VERSION: Document<Prose<String>> = Document::new("shelf", "version", 1, "The version.");

#[test]
fn a_document_named_other_than_its_command_path_is_a_document_name_customisation() {
    const GUIDE: Guide = Guide::new().leaves(&[
        Leaf::new(&["books", "show"]).document(BOOK_SHOW),
        Leaf::new(&["version"]).document(VERSION),
        Leaf::new(&["meta", "schemas"]).document(SCHEMAS_LIST),
    ]);
    let dir = TempDir::new("opt-outs-document-name").unwrap();

    run_guided(&GUIDE, &dir, Action::Overwrite);

    assert_eq!(
        read(&dir, "opt-outs.txt"),
        "shelf has no opt-outs from the conventions.\n\
         \n\
         shelf declares 1 customisation, which needs no reason.\n\
         \n\
         document-name: shelf books show\n\
         \x20 Declares: The command prints shelf/book-show/v1, where its path gives the name books-show.\n"
    );
}

#[test]
fn a_cargo_subcommand_names_its_documents_after_the_path_below_its_own_name() {
    const WEIGH: Document<Prose<String>> = Document::new("crumbkit", "weigh", 1, "The weights.");
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["crumbkit", "weigh"]).document(WEIGH)]);

    assert!(customisations(&Command::new("cargo"), &GUIDE).is_empty());
}

#[test]
fn the_dashed_forms_declaration_and_each_command_named_in_another_style_are_customisations() {
    let shelf = Command::new("shelf").subcommand(
        Command::new("reading_lists")
            .subcommand(Command::new("ExportAll"))
            .subcommand(Command::new("list")),
    );
    let listed = |guide: &Guide| -> Vec<_> {
        customisations(&shelf, guide)
            .into_iter()
            .map(|entry| (entry.declaration, entry.applies_to, entry.declares))
            .collect()
    };

    assert_eq!(
        listed(&Guide::new().dashed_forms()),
        [
            (
                "dashed-forms",
                "every run of shelf".to_owned(),
                "A command named in another style also answers to its dashed form.".to_owned()
            ),
            (
                "resource-name",
                "shelf reading_lists".to_owned(),
                "The command also answers to reading-lists.".to_owned()
            ),
            (
                "verb-name",
                "shelf reading_lists ExportAll".to_owned(),
                "The command also answers to export-all.".to_owned()
            ),
        ]
    );
    assert_eq!(listed(&Guide::new()), Vec::new());
}

#[test]
fn sorted_json_keys_are_a_customisation_of_every_run() {
    let listed = |guide: &Guide| -> Vec<_> {
        customisations(&Command::new("shelf"), guide)
            .into_iter()
            .map(|entry| (entry.declaration, entry.applies_to, entry.declares))
            .collect()
    };

    assert_eq!(
        listed(&Guide::new().json(JsonStyle::new().sorted())),
        [(
            "json-keys",
            "every run of shelf".to_owned(),
            "Every JSON document prints the keys of each object in sorted order.".to_owned()
        )]
    );
    assert_eq!(listed(&Guide::new()), Vec::new());
}

/// A book of the batch of `books show`.
#[derive(serde::Serialize, schemars::JsonSchema)]
struct Book {
    /// The book's id.
    id: u32,
}

impl crumbkit::Row for Book {
    const COLUMNS: &'static [&'static str] = &["id"];
}

#[test]
fn a_batch_that_prints_its_failures_in_input_order_is_a_failures_in_order_customisation() {
    const BOOKS_SHOW: Document<Batch<Book, crumbkit::Error>> =
        Document::new("shelf", "books-show", 1, "The books.").envelope(Envelope::WithoutInOrder);
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "show"]).document(BOOKS_SHOW)]);

    let list = customisations(&Command::new("shelf"), &GUIDE);

    let list = serde_json::to_value(&list).unwrap();
    assert_eq!(
        list,
        json!([{
            "declaration": "failures-in-order",
            "applies_to": "shelf books show",
            "declares": "With --json, the command prints its failures among its rows in input order, in place of text on standard error.",
        }])
    );
}

#[test]
fn a_document_with_keys_is_an_envelope_keys_customisation_before_its_failures_in_order() {
    const BOOKS_SHOW: Document<Batch<Book, crumbkit::Error>> =
        Document::new("shelf", "books-show", 1, "The books.").envelope(Envelope::KeysInOrder {
            type_key: "$type",
            data_key: "answers",
        });
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "show"]).document(BOOKS_SHOW)]);

    let list = customisations(&Command::new("shelf"), &GUIDE);

    let list = serde_json::to_value(&list).unwrap();
    assert_eq!(
        list,
        json!([
            {
                "declaration": "envelope-keys",
                "applies_to": "shelf books show",
                "declares": "With --json, the command prints shelf/books-show/v1 under $type and its result under answers, in place of schema and data.",
            },
            {
                "declaration": "failures-in-order",
                "applies_to": "shelf books show",
                "declares": "With --json, the command prints its failures among its rows in input order, in place of text on standard error.",
            },
        ])
    );
}

#[test]
fn a_batch_that_answers_each_left_out_key_is_a_left_out_per_key_customisation_after_its_failures_in_order()
 {
    const BOOKS_SHOW: Document<Batch<Book, crumbkit::Error>> =
        Document::new("shelf", "books-show", 1, "The books.")
            .envelope(Envelope::WithoutInOrder)
            .left_out_per_key();
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "show"]).document(BOOKS_SHOW)]);

    let list = customisations(&Command::new("shelf"), &GUIDE);

    let declarations: Vec<Value> = serde_json::to_value(&list)
        .unwrap()
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(declarations.len(), 2);
    assert_eq!(
        declarations[1],
        json!({
            "declaration": "left-out-per-key",
            "applies_to": "shelf books show",
            "declares": "When the output budget leaves out answers, the command prints one failure for each answer left out, at its key's position, in place of one failure that names them all. The JSON document can pass the budget by the size of those failures.",
        })
    );
}

#[test]
fn a_leaf_s_rename_and_an_exit_that_lists_it_are_customisations_of_that_command() {
    const MOVED: Code = Code::new("shelf", "books", "moved");
    const GUIDE: Guide = Guide::new()
        .exits(&[Exit::new(6, "A book moved during the import.").failures(&[&MOVED])])
        .leaves(&[Leaf::new(&["books", "import"])
            .codes(&[Rename::new(&crumbkit::codes::VERSION_CONFLICT, &MOVED)])]);
    let shelf = Command::new("shelf");

    let listed: Vec<_> = customisations(&shelf, &GUIDE)
        .into_iter()
        .map(|entry| (entry.declaration, entry.applies_to, entry.declares))
        .collect();

    assert_eq!(
        listed,
        [
            (
                "exit-code",
                "shelf books import".to_owned(),
                "A failure with shelf::books::moved exits with 6, where the default is 1."
                    .to_owned()
            ),
            (
                "code-rename",
                "shelf books import".to_owned(),
                "Runs print shelf::books::moved in place of shelf::version::conflict.".to_owned()
            ),
        ]
    );
}

#[test]
fn field_names_from_another_system_are_one_field_names_customisation_of_every_run() {
    const GUIDE: Guide = Guide::new()
        .codes(&[Rename::new(&CLI_USAGE, &USAGE)])
        .field_names(&["requestId", "Content-Type"]);

    let list = customisations(&Command::new("shelf"), &GUIDE);

    let list = serde_json::to_value(&list).unwrap();
    assert_eq!(
        list[1],
        json!({
            "declaration": "field-names",
            "applies_to": "every run of shelf",
            "declares": "Failures may print the fields requestId and Content-Type, whose names come from another system.",
        })
    );
    assert_eq!(list.as_array().unwrap().len(), 2);
}

/// A removed book whose text is its own line.
#[derive(serde::Serialize, schemars::JsonSchema)]
struct Removed {
    /// The book's id.
    id: u32,
}

impl crumbkit::Row for Removed {
    const COLUMNS: &'static [&'static str] = &["id"];
    const LINE: Option<&'static str> = Some("removed <ID>");
}

#[test]
fn rows_that_print_their_own_text_line_are_a_text_line_customisation() {
    const BOOKS_REMOVE: Document<Vec<Removed>> =
        Document::new("shelf", "books-remove", 1, "The removed books.");
    const GUIDE: Guide =
        Guide::new().leaves(&[Leaf::new(&["books", "remove"]).document(BOOKS_REMOVE)]);

    let list = customisations(&Command::new("shelf"), &GUIDE);

    let list = serde_json::to_value(&list).unwrap();
    assert_eq!(
        list,
        json!([{
            "declaration": "text-line",
            "applies_to": "shelf books remove",
            "declares": "The text prints each row as the line removed <ID> in place of a table row.",
        }])
    );
}

/// A book that prints whole in JSON and as its own line in text.
#[derive(serde::Serialize, schemars::JsonSchema)]
struct WholeLined {
    /// The book's id.
    id: u32,
}

impl crumbkit::Row for WholeLined {
    const COLUMNS: &'static [&'static str] = &["id"];
    const WHOLE: bool = true;
    const LINE: Option<&'static str> = Some("book <ID>");
}

#[test]
fn the_report_lists_the_rows_whole_opt_out_and_the_text_line_customisation_of_one_leaf() {
    const BOOKS_SHOW: Document<Vec<WholeLined>> =
        Document::new("shelf", "books-show", 1, "The books.");
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "show"]).document(BOOKS_SHOW)]);
    const ALLOW: &[crate::Allow] = &[crate::Allow::new(
        crate::Rule::RowsWhole,
        "books show",
        "Scripts read every field of each book.",
    )];
    let dir = TempDir::new("opt-outs-whole-lined").unwrap();

    super::super::check(
        &super::shelf(),
        &GUIDE,
        ALLOW,
        &crate::Budgets::new(),
        None,
        dir.path(),
        &snapbox::Assert::new().action(Action::Overwrite),
    );

    let text = read(&dir, "opt-outs.txt");
    assert!(
        text.contains("documents/rows-whole: shelf books show\n"),
        "{text}"
    );
    assert!(text.contains("text-line: shelf books show\n"), "{text}");
    let json: Value = serde_json::from_str(&read(&dir, "opt-outs.json")).unwrap();
    assert_eq!(json["data"][0]["rule"], json!("documents/rows-whole"));
    assert_eq!(json["customisations"][0]["declaration"], json!("text-line"));
}
