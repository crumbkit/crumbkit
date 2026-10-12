//! Tests for `schemas list` and `schemas show`, and for the JSON Schema of
//! a whole document.

use std::fmt;
use std::process::ExitCode;

use clap::{Args as _, Command};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Value, json};

use super::{
    SCHEMAS_LIST, SCHEMAS_SHOW, SchemasList, SchemasShow, document_schema, documents, list, show,
};
use crate::help::{Guide, Leaf};
use crate::{
    AsFailure as _, Batch, Document, Error, Listing, NoSummary, Output, Prose, Row, Sink, Spelling,
    finish_to,
};

mod formats;

/// A row type for the test tool's leaves.
#[derive(Serialize, JsonSchema)]
struct Book {
    /// The book's id.
    id: u32,
    /// The book's title.
    title: String,
}

impl Row for Book {
    const COLUMNS: &'static [&'static str] = &["id", "title"];
}

/// A summary of a listing of books.
#[derive(Serialize, JsonSchema)]
struct Shelved {
    /// The number of books on the shelf.
    shelved: u32,
}

impl fmt::Display for Shelved {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} shelved", self.shelved)
    }
}

const BOOKS_LIST: Document<Listing<Book>> =
    Document::new("shelf", "books-list", 1, "One page of the books.");
const BOOKS_SHOW: Document<Batch<Book, Error>> =
    Document::new("shelf", "books-show", 1, "The books for the ids given.");
const BOOKS_PURGE: Document<Vec<Book>> =
    Document::new("shelf", "books-purge", 1, "The books that were removed.");
const BOOKS_GONE: Document<Vec<Book>> = Document::new(
    "shelf",
    "books-gone",
    1,
    "The books of a command the tool lacks.",
);

/// A guide whose leaves name a hidden command, a command the tool lacks,
/// and two commands that print the same document.
const GUIDE: Guide = Guide::new().leaves(&[
    Leaf::new(&["books", "list"]).document(BOOKS_LIST),
    Leaf::new(&["books", "show"]).document(BOOKS_SHOW),
    Leaf::new(&["books", "purge"]).document(BOOKS_PURGE),
    Leaf::new(&["books", "gone"]).document(BOOKS_GONE),
    Leaf::new(&["books", "find"]).document(BOOKS_LIST),
    Leaf::new(&["schemas", "list"]).document(SCHEMAS_LIST),
    Leaf::new(&["schemas", "show"]).document(SCHEMAS_SHOW),
]);

/// A tool with the leaves of [`GUIDE`] apart from `books gone`, with
/// `books purge` hidden.
fn tool() -> Command {
    Command::new("shelf")
        .disable_help_subcommand(true)
        .subcommand(
            Command::new("books")
                .subcommand(Command::new("list"))
                .subcommand(Command::new("show"))
                .subcommand(Command::new("purge").hide(true))
                .subcommand(Command::new("find")),
        )
        .subcommand(
            Command::new("schemas")
                .subcommand(SchemasList::augment_args(Command::new("list")))
                .subcommand(SchemasShow::augment_args(Command::new("show"))),
        )
}

/// Returns the output flags that `flags` set, with the tool's command set
/// as `main` sets it.
fn output_with(flags: &[&str]) -> Output {
    let matches = Output::augment_args(Command::new("shelf"))
        .try_get_matches_from(std::iter::once(&"shelf").chain(flags))
        .unwrap();
    let mut output = <Output as clap::FromArgMatches>::from_arg_matches(&matches).unwrap();
    output.set_command(tool());
    output
}

/// Returns the default output flags with the tool's command set.
fn output() -> Output {
    output_with(&[])
}

/// Parses `words` after `shelf schemas <name>` into the arguments `A`.
fn args<A: clap::FromArgMatches>(name: &str, words: &[&str]) -> A {
    let mut command = tool();
    let matches = command
        .try_get_matches_from_mut(["shelf", "schemas", name].iter().chain(words))
        .unwrap();
    let (_, matches) = matches.subcommand().unwrap();
    let (_, matches) = matches.subcommand().unwrap();
    A::from_arg_matches(matches).unwrap()
}

/// What `finish_to` returned and wrote.
struct Finished {
    code: ExitCode,
    out: String,
    err: String,
}

/// Runs `shelf schemas show` with `words` and the output flags `output`,
/// and finishes the result into memory as a tool's handler does.
fn finish_show(output: &mut Output, words: &[&str]) -> Finished {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result = show(output, &GUIDE, &args::<SchemasShow>("show", words));
    let code = finish_to(
        &mut Sink::new(&mut out, &mut err),
        output,
        Spelling::COLONS,
        SCHEMAS_SHOW,
        result,
    );
    Finished {
        code,
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

/// Returns the names of the properties of `schema`, sorted.
fn properties(schema: &Value) -> Vec<&str> {
    let mut names: Vec<&str> = schema["properties"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    names.sort_unstable();
    names
}

#[test]
fn a_document_schema_names_the_id_as_its_title_and_the_const_of_schema() {
    let schema = document_schema(BOOKS_LIST).to_value();

    assert_eq!(
        schema["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    assert_eq!(schema["title"], "shelf/books-list/v1");
    assert_eq!(schema["description"], "One page of the books.");
    assert_eq!(schema["type"], "object");
    assert_eq!(
        schema["properties"]["schema"]["const"],
        "shelf/books-list/v1"
    );
}

#[test]
fn a_listing_without_a_summary_holds_data_and_total_and_no_summary() {
    let schema = document_schema(BOOKS_LIST).to_value();

    assert_eq!(properties(&schema), ["data", "dry_run", "schema", "total"]);
    assert_eq!(schema["required"], json!(["schema", "data", "total"]));
    assert_eq!(schema["properties"]["data"]["type"], "array");
    assert_eq!(
        schema["properties"]["data"]["items"]["$ref"],
        "#/$defs/Book"
    );
    assert_eq!(schema["properties"]["total"]["type"], "integer");
}

#[test]
fn a_listing_with_a_summary_requires_the_summary_and_holds_its_schema() {
    const SHELVED: Document<Listing<Book, Shelved>> =
        Document::new("shelf", "shelved", 1, "The shelved books.");

    let schema = document_schema(SHELVED).to_value();

    assert_eq!(
        properties(&schema),
        ["data", "dry_run", "schema", "summary", "total"]
    );
    assert_eq!(
        schema["required"],
        json!(["schema", "data", "total", "summary"])
    );
    assert_eq!(schema["properties"]["summary"]["$ref"], "#/$defs/Shelved");
    assert_eq!(
        schema["$defs"]["Shelved"]["properties"]["shelved"]["description"],
        "The number of books on the shelf."
    );
}

#[test]
fn a_single_row_document_holds_the_row_schema_under_data() {
    const BOOK: Document<Book> = Document::new("shelf", "book", 1, "One book.");

    let schema = document_schema(BOOK).to_value();

    assert_eq!(properties(&schema), ["data", "dry_run", "schema"]);
    assert_eq!(schema["required"], json!(["schema", "data"]));
    assert_eq!(schema["properties"]["data"]["$ref"], "#/$defs/Book");
}

#[test]
fn a_vec_document_holds_an_array_of_rows_under_data() {
    let schema = document_schema(BOOKS_PURGE).to_value();

    assert_eq!(properties(&schema), ["data", "dry_run", "schema"]);
    assert_eq!(schema["properties"]["data"]["type"], "array");
}

#[test]
fn a_batch_document_holds_its_failures_as_failure_documents_without_schema() {
    let schema = document_schema(BOOKS_SHOW).to_value();

    assert_eq!(
        properties(&schema),
        ["data", "dry_run", "failures", "schema"]
    );
    assert_eq!(schema["required"], json!(["schema", "data", "failures"]));
    let entry = &schema["properties"]["failures"]["items"];
    assert_eq!(entry["required"], json!(["code", "message", "recovery"]));
    assert!(entry["properties"].get("schema").is_none());
}

#[test]
fn a_prose_document_holds_the_schema_of_its_value_under_data_and_requires_failures() {
    const SHELF_SUMMARY: Document<Prose<Shelved>> =
        Document::new("shelf", "shelf-summary", 1, "What the shelf holds.");

    let schema = document_schema(SHELF_SUMMARY).to_value();

    assert_eq!(
        properties(&schema),
        ["data", "dry_run", "failures", "schema"]
    );
    assert_eq!(schema["required"], json!(["schema", "data", "failures"]));
    assert_eq!(schema["properties"]["data"]["$ref"], "#/$defs/Shelved");
    assert!(schema["$defs"].get("NoRow").is_none());
}

#[test]
fn the_row_type_schema_sits_under_defs_with_the_doc_comments_of_its_fields() {
    let schema = document_schema(BOOKS_LIST).to_value();

    assert_eq!(
        schema["$defs"]["Book"]["properties"]["title"]["description"],
        "The book's title."
    );
}

#[test]
fn the_schema_of_no_summary_never_reaches_a_document() {
    // A listing without a summary has `NoSummary`, whose schema is `false`.
    const PLAIN: Document<Listing<Book, NoSummary>> =
        Document::new("shelf", "plain", 1, "The plain books.");

    let schema = document_schema(PLAIN).to_value();

    assert!(schema["$defs"].get("NoSummary").is_none());
    assert!(schema["properties"].get("summary").is_none());
}

#[test]
fn the_failure_document_comes_first_and_then_each_leaf_document_in_guide_order() {
    let served = documents(&tool(), &GUIDE);

    let ids: Vec<String> = served.iter().map(super::Served::id).collect();
    assert_eq!(
        ids,
        [
            "crumbkit/failure/v1",
            "shelf/books-list/v1",
            "shelf/books-show/v1",
            "crumbkit/schemas-list/v1",
            "crumbkit/schemas-show/v1",
        ]
    );
}

#[test]
fn a_leaf_whose_command_is_hidden_or_missing_is_left_out() {
    let served = documents(&tool(), &GUIDE);

    assert!(
        served
            .iter()
            .all(|document| !document.id().contains("purge") && !document.id().contains("gone"))
    );
}

#[test]
fn a_document_that_two_leaves_print_is_served_once_with_both_command_lines() {
    let served = documents(&tool(), &GUIDE);

    let books_list = served
        .iter()
        .find(|document| document.id() == "shelf/books-list/v1")
        .unwrap();
    assert_eq!(
        books_list.commands(),
        ["shelf books list", "shelf books find"]
    );
    assert_eq!(books_list.description(), "One page of the books.");
}

#[test]
fn the_failure_document_names_no_command_and_has_the_checked_in_schema() {
    let served = documents(&tool(), &GUIDE);

    assert_eq!(served[0].commands(), [] as [String; 0]);
    assert_eq!(
        served[0].schema(),
        crate::output::failure::document_schema()
    );
}

#[test]
fn list_prints_one_row_for_each_served_document_with_its_description() {
    let listing = list(&mut output(), &GUIDE, &args::<SchemasList>("list", &[])).unwrap();

    let value = serde_json::to_value(&listing).unwrap();
    assert_eq!(
        value[1],
        json!({
            "id": "shelf/books-list/v1",
            "kind": "output",
            "description": "One page of the books.",
            "commands": ["shelf books list", "shelf books find"],
        })
    );
    assert_eq!(value.as_array().unwrap().len(), 5);
}

#[test]
fn list_prints_one_page_and_counts_every_document_in_its_total() {
    use crate::Rows as _;

    let listing = list(
        &mut output(),
        &GUIDE,
        &args::<SchemasList>("list", &["--limit", "2"]),
    )
    .unwrap();

    assert_eq!(listing.rows().len(), 2);
    assert_eq!(listing.total(), Some(5));
}

#[test]
fn list_fails_with_unavailable_when_the_tool_kept_no_command() {
    let error = list(
        &mut Output::default(),
        &GUIDE,
        &args::<SchemasList>("list", &[]),
    )
    .unwrap_err();

    let code = error.as_failure().code();
    assert_eq!((code.area(), code.reason()), ("schemas", "unavailable"));
}

#[test]
fn show_returns_the_schema_of_each_id_in_the_order_given() {
    let batch = show(
        &mut output(),
        &GUIDE,
        &args::<SchemasShow>("show", &["shelf/books-show/v1", "crumbkit/failure/v1"]),
    )
    .unwrap();

    let value = serde_json::to_value(&batch).unwrap();
    assert_eq!(value[0]["id"], "shelf/books-show/v1");
    assert_eq!(value[0]["schema"], document_schema(BOOKS_SHOW).to_value());
    assert_eq!(value[1]["id"], "crumbkit/failure/v1");
}

#[test]
fn show_fails_an_unknown_id_and_still_prints_the_ids_it_knows() {
    let finished = finish_show(
        &mut output_with(&["--json"]),
        &["shelf/books-lst/v1", "shelf/books-list/v1"],
    );

    assert_eq!(finished.code, ExitCode::FAILURE);
    let document: Value = serde_json::from_str(&finished.out).unwrap();
    assert_eq!(document["data"][0]["id"], "shelf/books-list/v1");
    assert_eq!(
        document["failures"],
        json!([{
            "code": "crumbkit::schemas::not_found",
            "message": "shelf has no document shelf/books-lst/v1",
            "recovery": {"kind": "run", "call": "shelf schemas list"},
        }])
    );
}

#[test]
fn show_in_text_prints_the_schema_as_one_line_of_json() {
    let finished = finish_show(&mut output(), &["shelf/books-list/v1"]);

    assert_eq!(finished.code, ExitCode::SUCCESS);
    let line = finished.out.lines().nth(1).unwrap();
    assert!(line.starts_with("shelf/books-list/v1  {\"$defs\":"));
    assert_eq!(finished.err, "");
}

#[test]
fn the_checked_in_schemas_match_the_schemas_of_the_two_documents() {
    for (document, checked_in) in [
        (
            document_schema(SCHEMAS_LIST),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/schemas/schemas-list-v1.json"
            )),
        ),
        (
            document_schema(SCHEMAS_SHOW),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/schemas/schemas-show-v1.json"
            )),
        ),
    ] {
        let generated = serde_json::to_string_pretty(&document).unwrap() + "\n";
        assert_eq!(
            checked_in, generated,
            "a file under schemas/ differs from the generated schema; replace the file with the generated schema"
        );
    }
}

#[test]
fn every_document_with_the_envelope_may_hold_dry_run_true_and_never_requires_it() {
    let schema = document_schema(BOOKS_LIST).to_value();

    assert_eq!(schema["properties"]["dry_run"]["const"], true);
    assert!(
        !schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("dry_run"))
    );
}
