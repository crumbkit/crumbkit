//! A tool outside crumbkit with value types of its own: a page count that
//! reads `432 pages` in text and `432` in JSON, an `Elapsed`, and child
//! lines from shown values, as the text table, `--json`, `--fields`, the
//! schema and a failure's fields print them.

use std::fmt;
use std::time::Duration as Std;

use clap::{Parser, Subcommand};
use crumbkit::schemars::{JsonSchema, Schema, SchemaGenerator};
use crumbkit::{
    Call, Children, Code, Document, Elapsed, Failure, Field, Recovery, Row, Run, Shown, Sink, Tool,
    ViewFlags,
};
use serde::{Serialize, Serializer};
use serde_json::{Value, json};

/// A tool with a `list` command.
#[derive(Parser)]
#[command(name = "loans")]
struct Cli {
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
struct Reminder(Elapsed);

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
    kept: Elapsed,
    /// The reminders sent about the loan.
    reminders: Vec<Reminder>,
}

impl Row for Loan {
    const COLUMNS: &'static [&'static str] = &["title", "pages", "kept"];
    const CHILDREN: Option<Children<Self>> = Some(Children::field("reminders"));
}

const LOANS: Document<Vec<Loan>> = Document::new("loans", "loans", 1, "The loans.");

const OVERDUE: Code<'static> = Code::new("loans", "loans", "overdue");

/// A loan kept past its term.
#[derive(Debug)]
struct Overdue {
    /// How long past its term the loan is.
    late: Elapsed,
}

impl fmt::Display for Overdue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the loan is {} past its term", self.late)
    }
}

impl std::error::Error for Overdue {}

impl Failure for Overdue {
    fn code(&self) -> Code<'_> {
        OVERDUE
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
        kept: Elapsed::new(Std::from_secs(15 * 24 * 3_600 + 3 * 3_600)),
        reminders: vec![Reminder(Elapsed::new(Std::from_secs(14 * 24 * 3_600)))],
    }
}

/// Parses `loans list` followed by `flags`, or returns `None` when the line
/// does not parse.
fn parse(flags: &[&str]) -> Option<Run> {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let words = ["loans", "list"].iter().chain(flags);
    let parsed = crumbkit::parse_to::<Cli>(&mut Sink::new(&mut out, &mut err), &Tool::new(), words);
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
        schema.pointer("/properties/kept/type"),
        Some(&json!("string"))
    );
    assert!(schema.pointer("/properties/kept/pattern").is_some());
    assert_eq!(
        schema.pointer("/properties/reminders/items"),
        Some(&json!({"type": "string"}))
    );
}

#[test]
fn a_failure_prints_a_shown_field_as_text_in_its_message_and_json_in_its_fields() {
    let late = Elapsed::new(Std::from_secs(3 * 24 * 3_600));

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

/// Runs `loans list` with `flags` on `rows` of any row type, and returns
/// standard output, or `None` when the run does not print.
fn print<R: Row>(flags: &[&str], rows: Vec<R>) -> Option<String> {
    let document: Document<Vec<R>> = Document::new("loans", "loans", 1, "The loans.");
    let output = parse(flags)?;
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result: Result<Vec<R>, Overdue> = Ok(rows);
    let code = crumbkit::finish_to(&mut Sink::new(&mut out, &mut err), output, document, result);
    assert_eq!(code, 0);
    String::from_utf8(out).ok()
}

/// Runs `loans list` on `rows` of any row type, and returns the text table,
/// or `None` when the run does not print.
fn table<R: Row>(rows: Vec<R>) -> Option<String> {
    print(&[], rows)
}

/// Runs `loans list --json` on `rows` of any row type, and returns the
/// document's data, or `None` when the run does not print JSON.
fn data<R: Row>(rows: Vec<R>) -> Option<Value> {
    let json: Value = serde_json::from_str(&print(&["--json"], rows)?).ok()?;
    json.get("data").cloned()
}

/// The length of a book, which a row flattens or a variant holds.
#[derive(Serialize, JsonSchema)]
struct Size {
    /// The book's length.
    pages: Pages,
}

/// A row that is an internally tagged enum.
#[derive(Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum Entry {
    /// A struct variant with a shown field.
    Book {
        /// The book's length.
        pages: Pages,
    },
    /// A newtype variant whose struct has a shown field.
    Issue(Size),
}

impl Row for Entry {
    const COLUMNS: &'static [&'static str] = &["kind", "pages"];
}

#[test]
fn a_row_that_is_an_internally_tagged_enum_shows_a_shown_field_as_its_display() {
    let rows = || {
        vec![
            Entry::Book { pages: Pages(432) },
            Entry::Issue(Size { pages: Pages(48) }),
        ]
    };

    let expected = "\
KIND   PAGES
book   432 pages
issue  48 pages
";
    assert_eq!(table(rows()).unwrap(), expected);
    assert_eq!(
        data(rows()),
        Some(json!([
            {"kind": "book", "pages": 432},
            {"kind": "issue", "pages": 48},
        ]))
    );
}

/// A row that flattens a struct with a shown field.
#[derive(Serialize, JsonSchema)]
struct Flat {
    /// The book's title.
    title: &'static str,
    /// The book's length.
    #[serde(flatten)]
    size: Size,
}

impl Row for Flat {
    const COLUMNS: &'static [&'static str] = &["title", "pages"];
}

#[test]
fn a_row_that_flattens_a_struct_shows_its_shown_field_as_its_display() {
    let rows = || {
        vec![Flat {
            title: "Emma",
            size: Size { pages: Pages(474) },
        }]
    };

    assert_eq!(table(rows()).unwrap(), "TITLE  PAGES\nEmma   474 pages\n");
    assert_eq!(data(rows()), Some(json!([{"title": "Emma", "pages": 474}])));
}

/// An enum whose struct variant has a shown field.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
enum Edition {
    /// A printed edition.
    Print {
        /// The edition's length.
        pages: Pages,
    },
}

/// A row that flattens an enum, whose struct variant serde buffers.
#[derive(Serialize, JsonSchema)]
struct FlatEdition {
    /// The book's title.
    title: &'static str,
    /// The book's edition.
    #[serde(flatten)]
    edition: Edition,
}

impl Row for FlatEdition {
    const COLUMNS: &'static [&'static str] = &["title", "print"];
}

/// A row that is an internally tagged enum whose newtype variant holds an
/// enum, whose struct variant serde buffers.
#[derive(Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum Tagged {
    /// A book in one edition.
    Book(Edition),
}

impl Row for Tagged {
    const COLUMNS: &'static [&'static str] = &["kind", "print"];
}

#[test]
fn a_shown_field_of_a_struct_variant_that_a_row_flattens_shows_its_display() {
    let rows = || {
        vec![FlatEdition {
            title: "Emma",
            edition: Edition::Print { pages: Pages(474) },
        }]
    };

    let expected = "\
TITLE  PRINT
Emma   {\"pages\":\"474 pages\"}
";
    assert_eq!(table(rows()).unwrap(), expected);
    assert_eq!(
        data(rows()),
        Some(json!([{"title": "Emma", "print": {"pages": 474}}]))
    );
}

#[test]
fn a_shown_field_of_a_struct_variant_inside_a_tagged_newtype_variant_shows_its_display() {
    let rows = || vec![Tagged::Book(Edition::Print { pages: Pages(474) })];

    let expected = "\
KIND  PRINT
book  {\"pages\":\"474 pages\"}
";
    assert_eq!(table(rows()).unwrap(), expected);
    assert_eq!(
        data(rows()),
        Some(json!([{"kind": "book", "print": {"pages": 474}}]))
    );
}
