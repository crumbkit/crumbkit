//! Tests for the problem facet of a row type.

use schemars::JsonSchema;
use serde::Serialize;

use crate::{Problem, Row};

/// A book whose `damage` field marks a problem when it holds a value.
#[derive(Serialize, JsonSchema)]
struct Book {
    /// The book's title.
    title: &'static str,
    /// What is wrong with the book, if anything.
    damage: serde_json::Value,
}

impl Row for Book {
    const COLUMNS: &'static [&'static str] = &["title", "damage"];
    const PROBLEM: Option<Problem<Self>> = Some(Problem::field("damage"));
}

/// Returns a book whose `damage` field holds `damage`.
fn book(damage: serde_json::Value) -> Book {
    Book {
        title: "Emma",
        damage,
    }
}

/// Reports whether the book with `damage` reports a problem.
fn damaged(damage: serde_json::Value) -> bool {
    Problem::<Book>::field("damage").reports(&book(damage))
}

#[test]
fn a_field_problem_reports_a_string_a_number_true_and_an_array() {
    assert!(damaged(serde_json::json!("torn")));
    assert!(damaged(serde_json::json!(3)));
    assert!(damaged(serde_json::json!(true)));
    assert!(damaged(serde_json::json!([])));
}

#[test]
fn a_field_problem_reports_nothing_for_null_false_or_an_empty_string() {
    assert!(!damaged(serde_json::Value::Null));
    assert!(!damaged(serde_json::json!(false)));
    assert!(!damaged(serde_json::json!("")));
}

#[test]
fn a_field_problem_reports_nothing_when_the_row_has_no_such_field() {
    let problem = Problem::<Book>::field("missing");
    assert!(!problem.reports(&book(serde_json::json!("torn"))));
}

#[test]
fn a_field_problem_names_its_field_as_its_column() {
    const PROBLEM: Problem<Book> = Problem::field("damage");
    assert_eq!(PROBLEM.get_column(), "damage");
    assert_eq!(PROBLEM.get_field(), Some("damage"));
}

/// A job whose state decides whether it reports a problem.
#[derive(Serialize, JsonSchema)]
struct Job {
    /// The job's name.
    name: &'static str,
    /// Whether the job failed.
    failed: bool,
}

impl Job {
    /// Reports whether the job failed.
    const fn failed(&self) -> bool {
        self.failed
    }
}

impl Row for Job {
    const COLUMNS: &'static [&'static str] = &["name", "failed"];
    const PROBLEM: Option<Problem<Self>> = Some(Problem::by("failed", Self::failed));
}

#[test]
fn a_problem_by_a_function_reports_what_the_function_returns() {
    const PROBLEM: Problem<Job> = Problem::by("failed", Job::failed);
    let job = |failed| Job {
        name: "fetch",
        failed,
    };
    assert!(PROBLEM.reports(&job(true)));
    assert!(!PROBLEM.reports(&job(false)));
}

#[test]
fn a_problem_by_a_function_names_its_column_and_no_field() {
    const PROBLEM: Problem<Job> = Problem::by("failed", Job::failed);
    assert_eq!(PROBLEM.get_column(), "failed");
    assert_eq!(PROBLEM.get_field(), None);
}

#[test]
fn a_problem_prints_its_column_and_rule_in_debug() {
    assert_eq!(
        format!("{:?}", Problem::<Book>::field("damage")),
        "Problem { column: \"damage\", rule: Field }"
    );
    assert_eq!(
        format!("{:?}", Problem::by("failed", Job::failed)),
        "Problem { column: \"failed\", rule: By }"
    );
}

/// A note with no problem facet.
#[derive(Serialize, JsonSchema)]
struct Note {
    /// The note's text.
    text: &'static str,
}

impl Row for Note {
    const COLUMNS: &'static [&'static str] = &["text"];
}

#[test]
fn a_row_type_without_a_problem_reports_none() {
    assert!(!super::super::problems::reports(&Note { text: "torn" }));
    assert!(super::super::problems::reports(&Job {
        name: "fetch",
        failed: true,
    }));
    assert!(super::super::problems::reports(&book(serde_json::json!(
        "torn"
    ))));
}
