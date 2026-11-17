//! Tests for `RowReports`.

use schemars::JsonSchema;
use serde::Serialize;

use crate::{Problem, Row, RowReports};

/// A book whose `damage` field marks a problem when it holds a value.
#[derive(Serialize, JsonSchema)]
struct Book {
    /// What is wrong with the book, if anything.
    damage: Option<&'static str>,
}

impl Row for Book {
    const COLUMNS: &'static [&'static str] = &["damage"];
    const PROBLEM: Option<Problem<Self>> = Some(Problem::field("damage"));
}

/// A job that a function decides has failed.
#[derive(Serialize, JsonSchema)]
struct Job {
    /// Whether the job failed.
    failed: bool,
}

impl Job {
    fn failed(&self) -> bool {
        self.failed
    }
}

impl Row for Job {
    const COLUMNS: &'static [&'static str] = &["failed"];
    const PROBLEM: Option<Problem<Self>> = Some(Problem::by("failed", Self::failed));
}

/// A note, which names no problem.
#[derive(Serialize, JsonSchema)]
struct Note {
    /// The note's text.
    text: &'static str,
}

impl Row for Note {
    const COLUMNS: &'static [&'static str] = &["text"];
}

#[test]
fn reports_problem_answers_as_the_row_types_problem_decides_for_field_and_by() {
    assert!(
        Book {
            damage: Some("torn")
        }
        .reports_problem()
    );
    assert!(!Book { damage: None }.reports_problem());
    assert!(Job { failed: true }.reports_problem());
    assert!(!Job { failed: false }.reports_problem());
}

#[test]
fn reports_problem_is_false_for_a_row_type_without_a_problem() {
    assert!(!Note { text: "torn" }.reports_problem());
}
