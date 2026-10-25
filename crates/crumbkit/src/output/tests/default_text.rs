//! Tests for a row type that decides which of its rows the default text of
//! a listing lists, with `Row::in_default_text` (ADR-0157).

use std::fmt;
use std::process::ExitCode;

use clap::CommandFactory as _;
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::Value;

use super::{Tool, built_with, finish_output, finish_with};
use crate::help::Guide;
use crate::{Document, Listing, Row};

/// An item of a cleanup run: a dry run removes nothing, and a real run
/// removes each item unless removing it needs the caller's own step.
#[derive(Serialize, JsonSchema)]
struct Item {
    /// The path of the item.
    path: &'static str,
    /// Whether the run removed the item.
    removed: bool,
    /// What the caller does to finish the item, when the run could not.
    step: Option<&'static str>,
    /// Whether the item could not be read.
    unreadable: bool,
    /// The size of the item in bytes.
    bytes: u64,
}

impl Row for Item {
    const COLUMNS: &'static [&'static str] = &["path", "removed", "step"];
    const LONG_COLUMNS: &'static [&'static str] = &["bytes"];
    const PROBLEM: Option<&'static str> = Some("unreadable");

    fn in_default_text(&self) -> bool {
        !self.removed || self.step.is_some()
    }
}

/// The summary line above the table.
#[derive(Serialize, JsonSchema)]
struct Removed {
    /// The number of items that the run removed.
    removed: usize,
}

impl fmt::Display for Removed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Removed {} items.", self.removed)
    }
}

const CLEANUP: Document<Listing<Item, Removed>> =
    Document::new("tidy", "cleanup", 1, "The test cleanup.");

const BARE_CLEANUP: Document<Listing<Item>> =
    Document::new("tidy", "bare-cleanup", 1, "The test cleanup.");

const ONLY_PROBLEMS: Document<Listing<Item>> =
    Document::new("tidy", "problem-cleanup", 1, "The test cleanup.").only_problems();

/// Returns an item at `path` that the run removed, or kept with `step`.
const fn item(path: &'static str, step: Option<&'static str>) -> Item {
    Item {
        path,
        removed: step.is_none(),
        step,
        unreadable: false,
        bytes: 0,
    }
}

/// Returns the items of a real run: one that needs a step, then two that
/// the run removed.
fn real_run() -> Vec<Item> {
    vec![
        item("cache/lock", Some("stop the server")),
        item("cache/a", None),
        item("cache/b", None),
    ]
}

/// Returns a listing of `items` with a summary that counts the removed ones.
fn summed(items: Vec<Item>) -> Listing<Item, Removed> {
    let removed = items.iter().filter(|item| item.removed).count();
    Listing::from_all(items, None)
        .described("items")
        .summary(Removed { removed })
}

#[test]
fn default_text_lists_only_the_rows_that_the_row_type_lists_by_default() {
    let finished = finish_with(&[], CLEANUP, summed(real_run()));

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(
        finished.out,
        "Removed 2 items.\nPATH        REMOVED  STEP\ncache/lock  false    stop the server\n"
    );
    assert_eq!(finished.err, "");
}

#[test]
fn a_dry_run_whose_rows_are_all_listed_by_default_prints_every_row() {
    let items = vec![
        Item {
            removed: false,
            ..item("cache/a", None)
        },
        Item {
            removed: false,
            ..item("cache/b", None)
        },
    ];

    let finished = finish_with(&[], CLEANUP, summed(items));

    assert_eq!(
        finished.out,
        "Removed 0 items.\nPATH     REMOVED\ncache/a  false\ncache/b  false\n"
    );
}

#[test]
fn long_and_fields_list_every_row() {
    for flags in [&["--long"][..], &["--fields", "path"][..]] {
        let finished = finish_with(flags, CLEANUP, summed(real_run()));

        assert_eq!(finished.out.matches("cache/").count(), 3, "{flags:?}");
    }
}

#[test]
fn json_holds_every_row() {
    let finished = finish_with(&["--json"], CLEANUP, summed(real_run()));

    let document: Value = serde_json::from_str(&finished.out).unwrap();
    assert_eq!(document["data"].as_array().unwrap().len(), 3);
}

#[test]
fn a_listing_whose_rows_are_all_left_out_prints_its_summary_alone_and_no_empty_note() {
    let items = vec![item("cache/a", None), item("cache/b", None)];

    let finished = finish_with(&[], CLEANUP, summed(items));

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(finished.out, "Removed 2 items.\n");
    assert_eq!(finished.err, "");
}

#[test]
fn a_listing_without_a_summary_whose_rows_are_all_left_out_prints_no_header() {
    let items = vec![item("cache/a", None), item("cache/b", None)];

    let finished = finish_with(&[], BARE_CLEANUP, Listing::from_all(items, None));

    assert_eq!(finished.out, "");
    assert_eq!(finished.err, "");
}

#[test]
fn a_listing_without_rows_or_a_summary_keeps_its_header_and_empty_note() {
    let finished = finish_with(&[], BARE_CLEANUP, Listing::from_all(Vec::new(), None));

    assert_eq!(finished.out, "PATH  REMOVED  STEP\n");
    assert_eq!(finished.err, "note: no rows match\n");
}

#[test]
fn a_row_that_reports_a_problem_is_listed_although_the_row_type_leaves_it_out() {
    let mut items = real_run();
    items.insert(
        0,
        Item {
            unreadable: true,
            ..item("cache/c", None)
        },
    );

    let finished = finish_with(&[], CLEANUP, summed(items));

    assert_eq!(finished.code, ExitCode::from(1));
    assert!(finished.out.contains("cache/c"), "{}", finished.out);
    assert!(!finished.out.contains("cache/a"), "{}", finished.out);
    assert_eq!(
        finished.err,
        "note: 1 of the items reports a problem, so the command exits with 1.\n"
    );
}

#[test]
fn only_problems_lists_no_row_that_reports_no_problem_whatever_the_row_type_says() {
    let finished = finish_with(&[], ONLY_PROBLEMS, Listing::from_all(real_run(), None));

    assert_eq!(finished.out, "");
    assert_eq!(finished.err, "");
}

/// A file that a cleanup run handled, which prints its own text line.
#[derive(Serialize, JsonSchema)]
struct File {
    /// The path of the file.
    path: &'static str,
    /// Whether the run removed the file.
    removed: bool,
}

impl Row for File {
    const COLUMNS: &'static [&'static str] = &["path", "removed"];
    const LINE: Option<&'static str> = Some("<PATH> (kept)");

    fn in_default_text(&self) -> bool {
        !self.removed
    }

    fn text_line(&self) -> Option<String> {
        Some(format!("{} (kept)", self.path))
    }
}

#[test]
fn rows_with_their_own_text_line_print_only_the_lines_of_the_rows_listed_by_default() {
    const FILES: Document<Listing<File>> = Document::new("tidy", "files", 1, "The test files.");
    let files = vec![
        File {
            path: "cache/lock",
            removed: false,
        },
        File {
            path: "cache/a",
            removed: true,
        },
    ];

    let finished = finish_with(&[], FILES, Listing::from_all(files, None));

    assert_eq!(finished.out, "cache/lock (kept)\n");
}

#[test]
fn a_list_of_rows_prints_every_row_in_its_default_text() {
    const ITEMS: Document<Vec<Item>> = Document::new("tidy", "items", 1, "The test items.");

    let finished = finish_with(&[], ITEMS, real_run());

    assert_eq!(
        finished.out.matches("cache/").count(),
        3,
        "{}",
        finished.out
    );
}

#[test]
fn the_footer_of_a_page_counts_every_row_of_the_page() {
    let page = Listing::new(real_run(), 5)
        .described("items")
        .summary(Removed { removed: 2 });

    let finished = finish_with(&[], CLEANUP, page);

    assert_eq!(
        finished.out.matches("cache/").count(),
        1,
        "{}",
        finished.out
    );
    assert_eq!(
        finished.err,
        "Showing 3 of 5 items. --all shows every one.\n"
    );
}

#[test]
fn the_budget_cut_line_counts_the_printed_rows_against_the_listing_total() {
    let kept = (0..10).map(|_| item("cache/lock", Some("stop the server")));
    let removed = (0..10).map(|_| item("cache/a", None));
    let listing = summed(kept.chain(removed).collect());
    let guide = Guide::new().output_budget(120, "the tests print little");
    let output = built_with(Tool::command(), &guide, &["tool", "list"]);

    let finished = finish_output(output, CLEANUP, listing);

    let rows = "cache/lock  false    stop the server\n".repeat(2);
    assert_eq!(
        finished.out,
        format!("Removed 10 items.\nPATH        REMOVED  STEP\n{rows}")
    );
    assert_eq!(
        finished.err,
        "Showing 2 of 20 items, because output stops at 120 bytes. \
         Ask for less with --limit or --fields.\n"
    );
}
