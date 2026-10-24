//! A tool outside crumbkit with value types of its own: a page count that
//! reads `432 pages` in text and `432` in JSON, a `Duration`, and child
//! lines from shown values, as the text table, `--json`, `--fields`, the
//! schema and a failure's fields print them.

use std::fmt;
use std::time::Duration as Std;

use clap::{CommandFactory as _, Parser, Subcommand};
use crumbkit::help::Guide;
use crumbkit::schemars::{JsonSchema, Schema, SchemaGenerator};
use crumbkit::{
    Call, Code, Document, Duration, Failure, Field, Output, Recovery, Row, Shown, Sink, ViewFlags,
};
use serde::{Serialize, Serializer};
use serde_json::{Value, json};

/// A tool with a `list` command.
#[derive(Parser)]
#[command(name = "loans")]
struct Tool {
    #[command(subcommand)]
    command: Command,
    #[command(flatten)]
    view: ViewFlags,
}

#[derive(Subcommand)]
enum Command {
    /// Lists the loans.
    List,
}

/// A number of pages, which reads `432 pages` in text and `432` in JSON.
#[derive(Clone, Copy)]
struct Pages(u32);

impl fmt::Display for Pages {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} pages", self.0)
    }
}

impl Shown for Pages {
    fn json<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(self.0)
    }

    fn schema(generator: &mut SchemaGenerator) -> Schema {
        u32::json_schema(generator)
    }
}

crumbkit::shown!(Pages);

/// A reminder sent about a loan, which prints as `reminded after 2w`.
struct Reminder(Duration);

impl fmt::Display for Reminder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "reminded after {}", self.0)
    }
}

impl Shown for Reminder {}

crumbkit::shown!(Reminder);

/// A book on loan.
#[derive(Serialize, JsonSchema)]
struct Loan {
    /// The book's title.
    title: &'static str,
    /// The book's length.
    pages: Pages,
    /// How long the reader has had the book.
    kept: Duration,
    /// The reminders sent about the loan.
    reminders: Vec<Reminder>,
}

impl Row for Loan {
    const COLUMNS: &'static [&'static str] = &["title", "pages", "kept"];
    const CHILDREN: Option<&'static str> = Some("reminders");
}

const LOANS: Document<Vec<Loan>> = Document::new("loans", "loans", 1, "The loans.");

const OVERDUE: Code = Code::new("loans", "loans", "overdue");

/// A loan kept past its term.
#[derive(Debug)]
struct Overdue {
    /// How long past its term the loan is.
    late: Duration,
}

impl fmt::Display for Overdue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the loan is {} past its term", self.late)
    }
}

impl std::error::Error for Overdue {}

impl Failure for Overdue {
    fn code(&self) -> &'static Code {
        &OVERDUE
    }

    fn recovery(&self) -> Recovery {
        Recovery::Run(Call::new(&["list"]))
    }

    fn fields(&self) -> Vec<Field<'_>> {
        vec![Field::shown("late", &self.late)]
    }
}

/// One loan of 432 pages kept for 15 days and 3 hours, with one reminder.
fn loan() -> Loan {
    Loan {
        title: "Pride and Prejudice",
        pages: Pages(432),
        kept: Duration(Std::from_secs(15 * 24 * 3_600 + 3 * 3_600)),
        reminders: vec![Reminder(Duration(Std::from_secs(14 * 24 * 3_600)))],
    }
}

/// Parses `loans list` followed by `flags`, or returns `None` when the line
/// does not parse.
fn parse(flags: &[&str]) -> Option<Output> {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let words = ["loans", "list"].iter().chain(flags);
    let parsed = crumbkit::parse_to::<Tool, _, _, _, _>(
        &mut Sink::new(&mut out, &mut err),
        Tool::command(),
        &Guide::new(),
        words,
    );
    parsed.ok().map(|(_tool, output)| output)
}

/// Runs `loans list` with `flags` on `result`, and returns the exit code,
/// standard output and standard error, or `None` when the line does not
/// parse or the output is not UTF-8.
fn run(flags: &[&str], result: Result<Vec<Loan>, Overdue>) -> Option<(u8, String, String)> {
    let output = parse(flags)?;
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = crumbkit::finish_to(&mut Sink::new(&mut out, &mut err), output, LOANS, result);
    Some((
        code,
        String::from_utf8(out).ok()?,
        String::from_utf8(err).ok()?,
    ))
}

/// Runs `loans list` with `flags` on one loan, and returns standard output,
/// or `None` when the run does not print.
fn list(flags: &[&str]) -> Option<String> {
    let (code, out, err) = run(flags, Ok(vec![loan()]))?;
    assert_eq!(code, 0, "{err}");
    Some(out)
}

#[test]
fn the_text_table_shows_each_shown_value_as_its_display_and_its_children_under_it() {
    let expected = "\
TITLE                PAGES      KEPT
Pride and Prejudice  432 pages  2w 1d
  reminded after 2w
";
    assert_eq!(list(&[]).unwrap(), expected);
}

#[test]
fn json_holds_each_shown_value_in_its_own_json_form() {
    let json: Value = serde_json::from_str(&list(&["--json"]).unwrap()).unwrap();

    assert_eq!(
        json.pointer("/data/0"),
        Some(&json!({
            "title": "Pride and Prejudice",
            "pages": 432,
            "kept": "PT363H",
            "reminders": ["reminded after 2w"],
        }))
    );
}

#[test]
fn fields_in_text_shows_the_display_and_fields_in_json_the_json_form() {
    assert_eq!(list(&["--fields", "pages"]).unwrap(), "PAGES\n432 pages\n");

    let json: Value =
        serde_json::from_str(&list(&["--json", "--fields", "pages,kept"]).unwrap()).unwrap();
    assert_eq!(
        json.pointer("/data/0"),
        Some(&json!({"pages": 432, "kept": "PT363H"}))
    );
}

#[test]
fn the_row_schema_describes_each_shown_value_in_its_json_form() {
    let schema = schemars::schema_for!(Loan).to_value();

    assert_eq!(
        schema.pointer("/properties/pages/type"),
        Some(&json!("integer"))
    );
    assert_eq!(
        schema.pointer("/properties/kept/format"),
        Some(&json!("duration"))
    );
    assert_eq!(
        schema.pointer("/properties/reminders/items"),
        Some(&json!({"type": "string"}))
    );
}

#[test]
fn a_failure_prints_a_shown_field_as_text_in_its_message_and_json_in_its_fields() {
    let late = Duration(Std::from_secs(3 * 24 * 3_600));

    let (code, _, err) = run(&[], Err(Overdue { late })).unwrap();
    assert_eq!(code, 1);
    assert!(err.contains("the loan is 3d past its term"), "{err}");

    let (code, out, _) = run(&["--json"], Err(Overdue { late })).unwrap();
    assert_eq!(code, 1);
    let json: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(
        json.pointer("/fields"),
        Some(&json!({"late": "PT72H"})),
        "{json}"
    );
}
