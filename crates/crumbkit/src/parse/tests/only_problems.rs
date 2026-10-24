//! Tests for a leaf whose document lists only the rows that report a
//! problem in its default text, and for `--long` and `--full` with
//! `--json` on a leaf whose row type sets `Row::WHOLE` (ADR-0138,
//! ADR-0140, ADR-0149).

use std::process::ExitCode;

use clap::{Args as _, Command};
use schemars::JsonSchema;
use serde::Serialize;

use super::parse_to;
use crate::help::{Guide, Leaf};
use crate::twins::assert_one_word_apart;
use crate::{Document, Listing, Row, Sink};

/// A check of one package, without long columns.
#[derive(Serialize, JsonSchema)]
struct Check {
    /// The package that was checked.
    package: &'static str,
    /// Whether the check passed.
    passed: bool,
}

impl Row for Check {
    const COLUMNS: &'static [&'static str] = &["package", "passed"];
    const WHOLE: bool = true;

    fn is_problem(&self) -> bool {
        !self.passed
    }
}

/// A row with a long column that a tool's readers expect whole.
#[derive(Serialize, JsonSchema)]
struct Shelf {
    /// The shelf's id.
    id: u32,
    /// The number of books on the shelf.
    books: u32,
}

impl Row for Shelf {
    const COLUMNS: &'static [&'static str] = &["id"];
    const LONG_COLUMNS: &'static [&'static str] = &["books"];
    const WHOLE: bool = true;
}

/// A row with a long column, whose JSON holds only its columns.
#[derive(Serialize, JsonSchema)]
struct Book {
    /// The book's id.
    id: u32,
    /// The number of pages.
    pages: u32,
}

impl Row for Book {
    const COLUMNS: &'static [&'static str] = &["id"];
    const LONG_COLUMNS: &'static [&'static str] = &["pages"];
}

/// A check whose readers expect it whole, with no columns for a table.
#[derive(Serialize, JsonSchema)]
struct Audit {
    /// The package that was audited.
    package: &'static str,
}

impl Row for Audit {
    const COLUMNS: &'static [&'static str] = &[];
    const WHOLE: bool = true;
}

const CHECKS: Document<Listing<Check>> =
    Document::new("tool", "checks", 1, "The checks.").only_problems();
const ALL_CHECKS: Document<Listing<Check>> = Document::new("tool", "all", 1, "The checks.");
const AUDITS: Document<Listing<Audit>> =
    Document::new("tool", "audits", 1, "The audits.").only_problems();
const SHELVES: Document<Listing<Shelf>> = Document::new("tool", "shelves", 1, "The shelves.");
const BOOKS: Document<Listing<Book>> = Document::new("tool", "books", 1, "The books.");

const GUIDE: Guide = Guide::new().leaves(&[
    Leaf::new(&["checks"]).document(CHECKS),
    Leaf::new(&["all"]).document(ALL_CHECKS),
    Leaf::new(&["shelves"]).document(SHELVES),
    Leaf::new(&["books"]).document(BOOKS),
    Leaf::new(&["audits"]).document(AUDITS),
]);

/// A tool with crumbkit's output flags on its root and the guide's leaves.
fn tool() -> Command {
    let leaves = ["checks", "all", "shelves", "books", "audits"]
        .map(|name| Command::new(name).about("A leaf"));
    crate::ViewFlags::augment_args(Command::new("tool"))
        .subcommand_required(true)
        .subcommands(leaves)
}

/// What `parse_to` printed, and its exit code when it did not return a
/// parsed type.
struct Parsed {
    code: Option<ExitCode>,
    out: String,
    err: String,
}

/// Runs `parse_to` over `tool()` with the guide's leaves and `args`.
fn parse(args: &[&str]) -> Parsed {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result =
        parse_to::<(), _, _, _, _>(&mut Sink::new(&mut out, &mut err), tool(), &GUIDE, args);
    Parsed {
        code: result.err().map(ExitCode::from),
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

const ROWS_LINE: &str = "Rows: the default text lists only the rows that report a problem, and --long, --fields or --json lists every row.";

#[test]
fn a_leaf_that_lists_only_problems_offers_long_although_its_rows_have_no_long_columns() {
    assert!(GUIDE.get_leaves()[0].offers(crate::help::ViewFlag::Long));
    assert!(!GUIDE.get_leaves()[1].offers(crate::help::ViewFlag::Long));

    let parsed = parse(&["tool", "checks", "--long"]);

    assert_eq!(parsed.code, None, "{}", parsed.err);
}

#[test]
fn the_help_of_a_leaf_that_lists_only_problems_says_that_long_fields_and_json_list_every_row() {
    let short = parse(&["tool", "checks", "-h"]);
    let long = parse(&["tool", "checks", "--help"]);

    assert!(
        short.out.contains(&format!("\n{ROWS_LINE}\n")),
        "{}",
        short.out
    );
    assert!(
        long.out.contains(&format!("\n{ROWS_LINE}\n")),
        "{}",
        long.out
    );
    assert!(long.out.contains("  --long\n"), "{}", long.out);
}

#[test]
fn the_rows_line_of_a_leaf_that_offers_neither_long_nor_fields_names_only_json() {
    let audits = &GUIDE.get_leaves()[4];
    assert!(!audits.offers(crate::help::ViewFlag::Long));
    assert!(!audits.offers(crate::help::ViewFlag::Fields));
    let short = parse(&["tool", "audits", "-h"]);
    let long = parse(&["tool", "audits", "--help"]);

    let line = "\nRows: the default text lists only the rows that report a problem, and --json lists every row.\n";
    assert!(short.out.contains(line), "{}", short.out);
    assert!(long.out.contains(line), "{}", long.out);
}

#[test]
fn the_help_of_a_leaf_that_lists_every_row_has_no_rows_line() {
    let parsed = parse(&["tool", "all", "--help"]);

    assert!(!parsed.out.contains("Rows:"), "{}", parsed.out);
}

#[test]
fn long_with_json_on_a_whole_row_leaf_is_accepted() {
    let parsed = parse(&["tool", "shelves", "--json", "--long"]);

    assert_eq!(parsed.code, None, "{}", parsed.out);
    assert_eq!(parsed.err, "");
}

#[test]
fn long_typed_before_the_command_path_with_json_on_a_whole_row_leaf_is_accepted() {
    let parsed = parse(&["tool", "--long", "checks", "--json"]);

    assert_eq!(parsed.code, None, "{}", parsed.out);
}

#[test]
fn full_with_json_on_a_whole_row_leaf_is_accepted() {
    assert!(!GUIDE.get_leaves()[2].offers(crate::help::ViewFlag::Full));

    let parsed = parse(&["tool", "shelves", "--json", "--full"]);

    assert_eq!(parsed.code, None, "{}", parsed.out);
}

#[test]
fn long_with_json_on_a_whole_row_leaf_without_columns_is_accepted() {
    let parsed = parse(&["tool", "audits", "--json", "--long"]);

    assert_eq!(parsed.code, None, "{}", parsed.out);
}

#[test]
fn long_without_json_on_a_whole_row_leaf_without_columns_is_refused() {
    let refused = ["tool", "audits", "--long"];
    let admitted = ["tool", "audits", "--json", "--long"];
    assert_one_word_apart(&refused, &admitted);

    let parsed = parse(&refused);

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    assert!(
        parsed
            .err
            .starts_with("error: --long adds the long columns of a table, and this command's rows have no columns\n"),
        "{}",
        parsed.err
    );
}

#[test]
fn long_without_json_on_a_whole_row_leaf_is_accepted() {
    let parsed = parse(&["tool", "shelves", "--long"]);

    assert_eq!(parsed.code, None, "{}", parsed.err);
}

#[test]
fn long_with_json_on_a_leaf_whose_json_holds_only_its_columns_is_accepted() {
    let parsed = parse(&["tool", "books", "--json", "--long"]);

    assert_eq!(parsed.code, None, "{}", parsed.err);
}
