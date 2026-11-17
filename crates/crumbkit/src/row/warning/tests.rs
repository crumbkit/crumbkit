//! Tests for the warning facet of a row type.

use schemars::JsonSchema;
use serde::Serialize;

use crate::{Problem, Row, RowReports, Warning};

/// A book whose `wear` field marks a warning when it holds a value.
#[derive(Serialize, JsonSchema)]
struct Book {
    /// The book's title.
    title: &'static str,
    /// How worn the book is, if at all.
    wear: serde_json::Value,
    /// What is wrong with the book, if anything.
    damage: serde_json::Value,
}

impl Row for Book {
    const COLUMNS: &'static [&'static str] = &["title", "wear", "damage"];
    const PROBLEM: Option<Problem<Self>> = Some(Problem::field("damage"));
    const WARNING: Option<Warning<Self>> = Some(Warning::field("wear"));
}

/// Returns a book with `wear` and `damage`.
fn book(wear: serde_json::Value, damage: serde_json::Value) -> Book {
    Book {
        title: "Emma",
        wear,
        damage,
    }
}

#[test]
fn a_warning_field_reports_a_value_other_than_null_false_or_an_empty_string() {
    let worn = |wear| Warning::<Book>::field("wear").reports(&book(wear, serde_json::Value::Null));
    assert!(worn(serde_json::json!("frayed")));
    assert!(worn(serde_json::json!(3)));
    assert!(worn(serde_json::json!(true)));
    assert!(!worn(serde_json::Value::Null));
    assert!(!worn(serde_json::json!(false)));
    assert!(!worn(serde_json::json!("")));
}

#[test]
fn a_warning_field_reports_nothing_when_the_row_has_no_such_field() {
    let warning = Warning::<Book>::field("missing");
    assert!(!warning.reports(&book(serde_json::json!("frayed"), serde_json::Value::Null)));
}

#[test]
fn a_warning_names_its_field_as_its_column() {
    const WARNING: Warning<Book> = Warning::field("wear");
    assert_eq!(WARNING.get_column(), "wear");
    assert_eq!(WARNING.get_field(), Some("wear"));
}

/// A job whose state decides whether it reports a warning.
#[derive(Serialize, JsonSchema)]
struct Job {
    /// Whether the job was retried.
    retried: bool,
}

impl Job {
    /// Reports whether the job was retried.
    const fn retried(&self) -> bool {
        self.retried
    }
}

#[test]
fn a_warning_by_a_function_reports_what_the_function_returns_and_names_no_field() {
    const WARNING: Warning<Job> = Warning::by("retried", Job::retried);
    assert!(WARNING.reports(&Job { retried: true }));
    assert!(!WARNING.reports(&Job { retried: false }));
    assert_eq!(WARNING.get_column(), "retried");
    assert_eq!(WARNING.get_field(), None);
}

#[test]
fn a_warning_prints_its_column_and_rule_in_debug() {
    assert_eq!(
        format!("{:?}", Warning::<Book>::field("wear")),
        "Warning { column: \"wear\", rule: Field }"
    );
    assert_eq!(
        format!("{:?}", Warning::by("retried", Job::retried)),
        "Warning { column: \"retried\", rule: By }"
    );
}

#[test]
fn a_row_that_reports_a_problem_and_a_warning_counts_only_as_a_problem() {
    let both = book(serde_json::json!("frayed"), serde_json::json!("torn"));
    assert!(both.reports_problem());
    assert!(!both.reports_warning());
}

#[test]
fn a_row_that_reports_only_a_warning_reports_a_warning_and_no_problem() {
    let worn = book(serde_json::json!("frayed"), serde_json::Value::Null);
    assert!(!worn.reports_problem());
    assert!(worn.reports_warning());
}

#[test]
fn a_row_that_reports_neither_reports_neither() {
    let fine = book(serde_json::Value::Null, serde_json::Value::Null);
    assert!(!fine.reports_problem());
    assert!(!fine.reports_warning());
}

/// A note, which names no warning.
#[derive(Serialize, JsonSchema)]
struct Note {
    /// The note's text.
    text: &'static str,
}

impl Row for Note {
    const COLUMNS: &'static [&'static str] = &["text"];
}

#[test]
fn a_row_type_that_names_no_warning_reports_none() {
    assert!(!Note { text: "hi" }.reports_warning());
}
