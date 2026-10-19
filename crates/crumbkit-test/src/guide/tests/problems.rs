//! Tests for the guide check of the field that `Row::PROBLEM` names.

use crumbkit::help::Guide;
use crumbkit::{Document, Row};
use schemars::JsonSchema;
use serde::Serialize;

use super::{check_guide, leaf, panic_message, shelf};

/// A book whose `problem` column marks a problem with text.
#[derive(Serialize, JsonSchema)]
struct CheckedBook {
    /// The book's id.
    id: u32,
    /// What is wrong with the book, or nothing.
    problem: Option<String>,
}

impl Row for CheckedBook {
    const COLUMNS: &'static [&'static str] = &["id", "problem"];
    const PROBLEM: Option<&'static str> = Some("problem");
}

/// A book whose problem field is only a long column.
#[derive(Serialize, JsonSchema)]
struct LongProblemBook {
    /// The book's id.
    id: u32,
    /// What is wrong with the book, or nothing.
    problem: String,
}

impl Row for LongProblemBook {
    const COLUMNS: &'static [&'static str] = &["id"];
    const LONG_COLUMNS: &'static [&'static str] = &["problem"];
    const PROBLEM: Option<&'static str> = Some("problem");
}

/// A book whose problem field is a number, which no value marks as no
/// problem.
#[derive(Serialize, JsonSchema)]
struct CountedBook {
    /// The book's id.
    id: u32,
    /// The number of problems.
    problems: u32,
}

impl Row for CountedBook {
    const COLUMNS: &'static [&'static str] = &["id", "problems"];
    const PROBLEM: Option<&'static str> = Some("problems");
}

#[test]
fn a_problem_field_that_is_a_compact_column_that_can_be_null_passes() {
    const GUIDE: Guide = Guide::new().leaves(&[
        leaf(&["books", "list"]).document(Document::<Vec<CheckedBook>>::new(
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
fn check_guide_fails_on_a_problem_field_that_is_not_a_compact_column() {
    const GUIDE: Guide = Guide::new().leaves(&[
        leaf(&["books", "list"]).document(Document::<Vec<LongProblemBook>>::new(
            "shelf",
            "books-list",
            1,
            "The test rows.",
        )),
        leaf(&["books", "show"]),
        leaf(&["authors", "list"]),
    ]);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE, &[])).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         shelf books list: the problem field `problem` is not one of the row type's COLUMNS, so the compact table hides the problem that decides the exit"
    );
}

#[test]
fn check_guide_fails_on_a_problem_field_whose_type_is_a_number() {
    const GUIDE: Guide = Guide::new().leaves(&[
        leaf(&["books", "list"]).document(Document::<Vec<CountedBook>>::new(
            "shelf",
            "books-list",
            1,
            "The test rows.",
        )),
        leaf(&["books", "show"]),
        leaf(&["authors", "list"]),
    ]);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE, &[])).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         shelf books list: the problem field `problems` is not a string, a boolean or a value that can be null, so no value of it means that the row has no problem"
    );
}
