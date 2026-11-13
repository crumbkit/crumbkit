//! Tests for the default child lines of a row.

use std::fmt;

use schemars::JsonSchema;
use serde::Serialize;

use crate::{Children, Elapsed, Row, Shown};

/// Returns the compact child lines of `row`, as its type's
/// [`Row::CHILDREN`] builds them, and no lines without it.
fn lines<R: Row>(row: &R) -> Vec<String> {
    R::CHILDREN
        .map(|children| children.lines_for(row))
        .unwrap_or_default()
}

/// One step of a job, which prints as `fetch: 3s`.
struct Step {
    /// The step's name.
    name: &'static str,
    /// How long the step took.
    took: Elapsed,
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
    const CHILDREN: Option<Children<Self>> = Some(Children::field("steps"));
}

fn job() -> Job {
    Job {
        name: "nightly",
        steps: vec![
            Step {
                name: "fetch",
                took: Elapsed::new(std::time::Duration::from_secs(3)),
            },
            Step {
                name: "build",
                took: Elapsed::new(std::time::Duration::from_secs(250)),
            },
        ],
    }
}

#[test]
fn a_children_field_of_shown_values_gives_one_line_for_each_value() {
    assert_eq!(lines(&job()), ["fetch: 3s", "build: 4m 10s"]);
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
    const CHILDREN: Option<Children<Self>> = Some(Children::field("tags"));
}

#[test]
fn a_children_field_of_strings_gives_one_line_for_each_string() {
    let row = Tagged {
        name: "a",
        tags: vec!["red", "old"],
        notes: vec!["unread"],
    };

    assert_eq!(lines(&row), ["red", "old"]);
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
    const CHILDREN: Option<Children<Self>> = Some(Children::field("parts"));
}

#[test]
fn a_children_field_of_objects_gives_no_lines() {
    let row = Machine {
        name: "press",
        parts: vec![Part { name: "belt" }],
    };

    assert_eq!(lines(&row), Vec::<String>::new());
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
    const CHILDREN: Option<Children<Self>> = Some(Children::field("items"));
}

#[test]
fn a_children_field_with_an_item_that_is_not_a_string_gives_no_lines() {
    let row = Mixed {
        name: "m",
        items: vec![serde_json::json!("one"), serde_json::json!(2)],
    };

    assert_eq!(lines(&row), Vec::<String>::new());
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

    assert_eq!(lines(&row), Vec::<String>::new());
}

/// A row whose children field names a field it does not have.
#[derive(Serialize, JsonSchema)]
struct Misnamed {
    /// The row's tags.
    tags: Vec<&'static str>,
}

impl Row for Misnamed {
    const COLUMNS: &'static [&'static str] = &["tags"];
    const CHILDREN: Option<Children<Self>> = Some(Children::field("labels"));
}

#[test]
fn a_children_field_that_the_row_lacks_gives_no_lines() {
    let row = Misnamed { tags: vec!["red"] };

    assert_eq!(lines(&row), Vec::<String>::new());
}

/// A row whose child lines count its tags.
#[derive(Serialize, JsonSchema)]
struct Counted {
    /// The row's tags.
    tags: Vec<&'static str>,
}

impl Counted {
    fn count(&self) -> Vec<String> {
        vec![format!("{} tags", self.tags.len())]
    }

    fn all(&self) -> Vec<String> {
        vec![self.tags.join(" and ")]
    }
}

#[test]
fn each_view_prints_the_lines_its_function_builds_and_otherwise_the_compact_lines() {
    let row = Counted {
        tags: vec!["red", "old"],
    };
    let both =
        |children: Children<Counted>| [children.lines_for(&row), children.long_lines_for(&row)];
    let field = Children::field("tags");

    assert_eq!(both(field), [vec!["red", "old"], vec!["red", "old"]]);
    assert_eq!(
        both(field.lines(Counted::count)),
        [vec!["2 tags"], vec!["2 tags"]]
    );
    assert_eq!(
        both(field.long_lines(Counted::all)),
        [vec!["red", "old"], vec!["red and old"]]
    );
    assert_eq!(
        both(field.lines(Counted::count).long_lines(Counted::all)),
        [vec!["2 tags"], vec!["red and old"]]
    );
}

#[test]
fn children_keep_the_defaults_until_a_builder_changes_them() {
    const PLAIN: Children<()> = Children::field("notes");
    const SET: Children<()> = Children::field("notes")
        .indent(0)
        .spaced()
        .help("one line for each note.")
        .lines(|()| Vec::new())
        .untyped();

    assert_eq!(
        (PLAIN.get_indent(), PLAIN.is_spaced(), PLAIN.get_help()),
        (2, false, None)
    );
    assert_eq!(
        (SET.get_indent(), SET.is_spaced(), SET.get_help()),
        (0, true, Some("one line for each note."))
    );
    assert_eq!(SET.get_field(), "notes");
}

#[test]
fn children_describe_their_lines_with_crumbkits_sentence_or_the_tools_help() {
    assert_eq!(
        Children::<()>::field("notes").text(),
        "one indented line under each row for each of its notes."
    );
    assert_eq!(
        Children::<()>::field("notes").indent(0).text(),
        "one line under each row for each of its notes."
    );
    assert_eq!(
        Children::<()>::field("notes")
            .help("one line for each note.")
            .text(),
        "one line for each note."
    );
}
