//! Tests for the names that crumbkit's own codes, documents and variables
//! take in a tool's output.

use super::ToolName;
use crate::{Code, Document, Spelling};

const USAGE: Code = Code::new("crumbkit", "cli", "usage");
const NOT_FOUND: Code = Code::new("shelf", "books", "not_found");
const FAILURE: Document<()> = Document::new("crumbkit", "failure", 1, "A failure.");
const BOOKS_LIST: Document<()> = Document::new("shelf", "books-list", 1, "The books.");

#[test]
fn a_name_that_is_a_code_component_spells_every_kind_of_name_as_it_is() {
    let tool = ToolName::new("shelf");

    assert_eq!(
        Spelling::COLONS.spell(&tool.code(&USAGE)),
        "shelf::cli::usage"
    );
    assert_eq!(tool.document(FAILURE).to_string(), "shelf/failure/v1");
    assert_eq!(tool.schema_id("crumbkit/failure/v1"), "shelf/failure/v1");
    assert_eq!(tool.variable("OUTPUT_BUDGET"), "SHELF_OUTPUT_BUDGET");
    assert_eq!(tool.name(), "shelf");
}

#[test]
fn a_name_with_a_hyphen_keeps_it_in_a_schema_id_and_spells_codes_and_variables_with_an_underscore()
{
    let tool = ToolName::new("my-tool");

    assert_eq!(
        Spelling::COLONS.spell(&tool.code(&USAGE)),
        "my_tool::cli::usage"
    );
    assert_eq!(tool.document(FAILURE).to_string(), "my-tool/failure/v1");
    assert_eq!(tool.variable("OUTPUT_BUDGET"), "MY_TOOL_OUTPUT_BUDGET");
    assert_eq!(tool.name(), "my-tool");
}

#[test]
fn a_name_with_capitals_or_other_characters_is_lowercased_and_each_other_character_replaced() {
    let tool = ToolName::new("My Tool.v2");

    assert_eq!(
        Spelling::COLONS.spell(&tool.code(&USAGE)),
        "my_tool_v2::cli::usage"
    );
    assert_eq!(tool.document(FAILURE).to_string(), "my-tool.v2/failure/v1");
    assert_eq!(tool.variable("OUTPUT_BUDGET"), "MY_TOOL_V2_OUTPUT_BUDGET");
}

#[test]
fn a_name_with_no_letter_or_digit_falls_back_to_tool() {
    let tool = ToolName::new("");

    assert_eq!(
        Spelling::COLONS.spell(&tool.code(&USAGE)),
        "tool::cli::usage"
    );
    assert_eq!(tool.document(FAILURE).to_string(), "tool/failure/v1");
    assert_eq!(tool.variable("OUTPUT_BUDGET"), "TOOL_OUTPUT_BUDGET");
}

#[test]
fn a_tools_own_code_document_and_schema_id_keep_their_component() {
    let tool = ToolName::new("ledger");

    assert_eq!(tool.code(&NOT_FOUND), NOT_FOUND);
    assert_eq!(tool.document(BOOKS_LIST).to_string(), "shelf/books-list/v1");
    assert_eq!(tool.schema_id("shelf/books-list/v1"), "shelf/books-list/v1");
}

#[test]
fn the_same_name_gives_equal_tool_names() {
    assert_eq!(ToolName::new("shelf"), ToolName::new("shelf"));
    assert_ne!(ToolName::new("shelf"), ToolName::new("ledger"));
}

#[test]
fn a_spelling_for_a_tool_spells_crumbkits_codes_with_the_tools_component() {
    let spelling = Spelling::DOTS.for_tool(ToolName::new("shelf"));

    assert_eq!(spelling.spell(&USAGE), "shelf.cli.usage");
    assert_eq!(spelling.spell(&NOT_FOUND), "shelf.books.not_found");
    assert_eq!(spelling.tool(), ToolName::new("shelf"));
}

#[test]
fn a_spelling_without_a_tool_spells_crumbkits_codes_as_they_are() {
    assert_eq!(Spelling::COLONS.spell(&USAGE), "crumbkit::cli::usage");
    assert_eq!(Spelling::COLONS.tool(), ToolName::CRUMBKIT);
}

#[test]
fn a_custom_spelling_for_a_tool_reads_the_tools_component() {
    fn upper(code: &Code, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", code.component().to_uppercase(), code.reason())
    }
    let spelling = Spelling::custom(upper).for_tool(ToolName::new("shelf"));

    assert_eq!(spelling.spell(&USAGE), "SHELF-usage");
}
