//! Tests for the tool record check of the column that `Row::WARNING` names.

use crumbkit::help::Help;
use crumbkit::{Document, Problem, Row, Tool, Warning};
use schemars::JsonSchema;
use serde::Serialize;

use super::{check_help, leaf, panic_message, shelf};

/// A book whose `wear` column marks a warning with text.
#[derive(Serialize, JsonSchema)]
struct WornBook {
    /// The book's id.
    id: u32,
    /// How worn the book is, or nothing.
    wear: Option<String>,
}

impl Row for WornBook {
    const COLUMNS: &'static [&'static str] = &["id", "wear"];
    const WARNING: Option<Warning<Self>> = Some(Warning::field("wear"));
}

/// A book whose warning field is only a long column.
#[derive(Serialize, JsonSchema)]
struct LongWearBook {
    /// The book's id.
    id: u32,
    /// How worn the book is, or nothing.
    wear: String,
}

impl Row for LongWearBook {
    const COLUMNS: &'static [&'static str] = &["id"];
    const LONG_COLUMNS: &'static [&'static str] = &["wear"];
    const WARNING: Option<Warning<Self>> = Some(Warning::field("wear"));
}

/// A book whose warning field is a number.
#[derive(Serialize, JsonSchema)]
struct CountedBook {
    /// The book's id.
    id: u32,
    /// The number of tears.
    tears: u32,
}

impl Row for CountedBook {
    const COLUMNS: &'static [&'static str] = &["id", "tears"];
    const WARNING: Option<Warning<Self>> = Some(Warning::field("tears"));
}

/// A book whose problem and warning name the same field.
#[derive(Serialize, JsonSchema)]
struct SameFieldBook {
    /// The book's id.
    id: u32,
    /// What is wrong with the book, or nothing.
    damage: Option<String>,
}

impl Row for SameFieldBook {
    const COLUMNS: &'static [&'static str] = &["id", "damage"];
    const PROBLEM: Option<Problem<Self>> = Some(Problem::field("damage"));
    const WARNING: Option<Warning<Self>> = Some(Warning::field("damage"));
}

/// A book whose problem and warning name different fields.
#[derive(Serialize, JsonSchema)]
struct TwoFieldBook {
    /// The book's id.
    id: u32,
    /// What is wrong with the book, or nothing.
    damage: Option<String>,
    /// How worn the book is, or nothing.
    wear: Option<String>,
}

impl Row for TwoFieldBook {
    const COLUMNS: &'static [&'static str] = &["id", "damage", "wear"];
    const PROBLEM: Option<Problem<Self>> = Some(Problem::field("damage"));
    const WARNING: Option<Warning<Self>> = Some(Warning::field("wear"));
}

/// Checks `tool` against a shelf whose `books list` prints `R`.
macro_rules! tool_of {
    ($row:ty) => {
        Tool::new().help(Help::new().leaves(&[
            leaf(&["books", "list"]).document(Document::<Vec<$row>>::new(
                "shelf",
                "books-list",
                1,
                "The test rows.",
            )),
            leaf(&["books", "show"]),
            leaf(&["authors", "list"]),
        ]))
    };
}

#[test]
fn a_warning_field_that_is_a_compact_column_that_can_be_null_passes() {
    const TOOL: Tool = tool_of!(WornBook);

    check_help(&shelf(), &TOOL, &[]);
}

#[test]
fn a_problem_and_a_warning_on_different_fields_pass() {
    const TOOL: Tool = tool_of!(TwoFieldBook);

    check_help(&shelf(), &TOOL, &[]);
}

#[test]
fn check_help_fails_a_warning_column_that_is_not_a_compact_column() {
    const TOOL: Tool = tool_of!(LongWearBook);

    assert_eq!(
        panic_message(|| check_help(&shelf(), &TOOL, &[])).unwrap(),
        "the command tree breaks crumbkit's help rules:\n\
         shelf books list: the warning field `wear` is not one of the row type's COLUMNS, so the compact table hides the warning that the note counts"
    );
}

#[test]
fn check_help_fails_a_warning_field_whose_type_is_a_number() {
    const TOOL: Tool = tool_of!(CountedBook);

    assert_eq!(
        panic_message(|| check_help(&shelf(), &TOOL, &[])).unwrap(),
        "the command tree breaks crumbkit's help rules:\n\
         shelf books list: the warning field `tears` is not a string, a boolean or a value that can be null, so no value of it means that the row has no warning"
    );
}

#[test]
fn check_help_fails_a_warning_field_that_is_also_the_problem_field() {
    const TOOL: Tool = tool_of!(SameFieldBook);

    assert_eq!(
        panic_message(|| check_help(&shelf(), &TOOL, &[])).unwrap(),
        "the command tree breaks crumbkit's help rules:\n\
         shelf books list: the warning field `damage` is also the problem field, so every row that reports a warning reports a problem and no row counts as a warning; name another field in `Warning::field`"
    );
}
