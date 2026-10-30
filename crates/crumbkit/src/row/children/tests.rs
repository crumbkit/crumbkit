//! Tests for the default child lines of a row.

use std::fmt;

use schemars::JsonSchema;
use serde::Serialize;

use crate::{Duration, Row, Shown};

/// One step of a job, which prints as `fetch: 3s`.
struct Step {
    /// The step's name.
    name: &'static str,
    /// How long the step took.
    took: Duration,
}

impl fmt::Display for Step {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.took)
    }
}

impl Shown for Step {}

crate::shown!(Step);

/// A job and its steps.
#[derive(Serialize, JsonSchema)]
struct Job {
    /// The job's name.
    name: &'static str,
    /// The job's steps.
    steps: Vec<Step>,
}

impl Row for Job {
    const COLUMNS: &'static [&'static str] = &["name"];
    const CHILDREN: Option<&'static str> = Some("steps");
}

fn job() -> Job {
    Job {
        name: "nightly",
        steps: vec![
            Step {
                name: "fetch",
                took: Duration::new(std::time::Duration::from_secs(3)),
            },
            Step {
                name: "build",
                took: Duration::new(std::time::Duration::from_secs(250)),
            },
        ],
    }
}

#[test]
fn a_children_field_of_shown_values_gives_one_line_for_each_value() {
    assert_eq!(job().child_lines(), ["fetch: 3s", "build: 4m 10s"]);
}

#[test]
fn the_default_long_child_lines_are_the_child_lines() {
    assert_eq!(job().long_child_lines(), ["fetch: 3s", "build: 4m 10s"]);
}

#[test]
fn the_rows_json_still_holds_the_json_form_of_its_children() {
    assert_eq!(
        serde_json::to_value(job()).unwrap(),
        serde_json::json!({"name": "nightly", "steps": ["fetch: 3s", "build: 4m 10s"]})
    );
}

/// A row whose children are strings, with a field that is not its children.
#[derive(Serialize, JsonSchema)]
struct Tagged {
    /// The row's name.
    name: &'static str,
    /// The row's tags.
    tags: Vec<&'static str>,
    /// The row's notes.
    notes: Vec<&'static str>,
}

impl Row for Tagged {
    const COLUMNS: &'static [&'static str] = &["name"];
    const CHILDREN: Option<&'static str> = Some("tags");
}

#[test]
fn a_children_field_of_strings_gives_one_line_for_each_string() {
    let row = Tagged {
        name: "a",
        tags: vec!["red", "old"],
        notes: vec!["unread"],
    };

    assert_eq!(row.child_lines(), ["red", "old"]);
}

/// A part of a row whose children serialize as objects.
#[derive(Serialize, JsonSchema)]
struct Part {
    /// The part's name.
    name: &'static str,
}

/// A row whose children are objects.
#[derive(Serialize, JsonSchema)]
struct Machine {
    /// The machine's name.
    name: &'static str,
    /// The machine's parts.
    parts: Vec<Part>,
}

impl Row for Machine {
    const COLUMNS: &'static [&'static str] = &["name"];
    const CHILDREN: Option<&'static str> = Some("parts");
}

#[test]
fn a_children_field_of_objects_gives_no_lines() {
    let row = Machine {
        name: "press",
        parts: vec![Part { name: "belt" }],
    };

    assert_eq!(row.child_lines(), Vec::<String>::new());
}

/// A row whose children field holds strings and numbers.
#[derive(Serialize, JsonSchema)]
struct Mixed {
    /// The row's name.
    name: &'static str,
    /// The row's items.
    items: Vec<serde_json::Value>,
}

impl Row for Mixed {
    const COLUMNS: &'static [&'static str] = &["name"];
    const CHILDREN: Option<&'static str> = Some("items");
}

#[test]
fn a_children_field_with_an_item_that_is_not_a_string_gives_no_lines() {
    let row = Mixed {
        name: "m",
        items: vec![serde_json::json!("one"), serde_json::json!(2)],
    };

    assert_eq!(row.child_lines(), Vec::<String>::new());
}

/// A row without children.
#[derive(Serialize, JsonSchema)]
struct Plain {
    /// The row's tags.
    tags: Vec<&'static str>,
}

impl Row for Plain {
    const COLUMNS: &'static [&'static str] = &["tags"];
}

#[test]
fn a_row_without_a_children_field_gives_no_lines() {
    let row = Plain { tags: vec!["red"] };

    assert_eq!(row.child_lines(), Vec::<String>::new());
}

/// A row whose children field names a field it does not have.
#[derive(Serialize, JsonSchema)]
struct Misnamed {
    /// The row's tags.
    tags: Vec<&'static str>,
}

impl Row for Misnamed {
    const COLUMNS: &'static [&'static str] = &["tags"];
    const CHILDREN: Option<&'static str> = Some("labels");
}

#[test]
fn a_children_field_that_the_row_lacks_gives_no_lines() {
    let row = Misnamed { tags: vec!["red"] };

    assert_eq!(row.child_lines(), Vec::<String>::new());
}

/// A row that writes its own child lines.
#[derive(Serialize, JsonSchema)]
struct Counted {
    /// The row's tags.
    tags: Vec<&'static str>,
}

impl Row for Counted {
    const COLUMNS: &'static [&'static str] = &["tags"];
    const CHILDREN: Option<&'static str> = Some("tags");

    fn child_lines(&self) -> Vec<String> {
        vec![format!("{} tags", self.tags.len())]
    }
}

#[test]
fn a_row_that_overrides_its_child_lines_keeps_its_own() {
    let row = Counted {
        tags: vec!["red", "old"],
    };

    assert_eq!(row.child_lines(), ["2 tags"]);
}
