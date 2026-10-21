//! Tests for a leaf whose listing document declares a long note: it offers
//! `--long` although its rows have no long columns, and its help says that
//! `--long` prints the note (ADR-0139).

use std::process::ExitCode;

use clap::{Args as _, Command};
use schemars::JsonSchema;
use serde::Serialize;

use super::parse_to;
use crate::help::{Guide, Leaf, ViewFlag};
use crate::twins::assert_one_word_apart;
use crate::{Document, Listing, Row, Sink};

/// The disk space that one directory takes, without long columns.
#[derive(Serialize, JsonSchema)]
struct Usage {
    /// The directory that was measured.
    path: &'static str,
    /// The bytes that its files take on disk.
    bytes: u64,
}

impl Row for Usage {
    const COLUMNS: &'static [&'static str] = &["path", "bytes"];
}

const NOTED: Document<Listing<Usage>> =
    Document::new("du", "noted", 1, "The space of each directory.").long_noted();
const PLAIN: Document<Listing<Usage>> =
    Document::new("du", "plain", 1, "The space of each directory.");

const GUIDE: Guide = Guide::new().leaves(&[
    Leaf::new(&["noted"]).document(NOTED),
    Leaf::new(&["plain"]).document(PLAIN),
]);

/// A tool with crumbkit's output flags on its root and the guide's leaves.
fn tool() -> Command {
    let leaves = ["noted", "plain"].map(|name| Command::new(name).about("A leaf"));
    crate::ViewFlags::augment_args(Command::new("du"))
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

const LONG_NOTE_LINE: &str = "Long note: --long prints a note on standard error.";

#[test]
fn a_leaf_whose_document_declares_a_long_note_offers_long_although_its_rows_have_no_long_columns() {
    assert!(GUIDE.get_leaves()[0].offers(ViewFlag::Long));
    assert!(!GUIDE.get_leaves()[1].offers(ViewFlag::Long));

    let parsed = parse(&["du", "noted", "--long"]);

    assert_eq!(parsed.code, None, "{}", parsed.err);
}

#[test]
fn a_leaf_whose_document_declares_no_long_note_refuses_long_when_its_rows_have_no_long_columns() {
    let refused = ["du", "plain", "--long"];
    let admitted = ["du", "plain"];
    assert_one_word_apart(&refused, &admitted);
    assert_eq!(parse(&admitted).code, None);

    let parsed = parse(&refused);

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    assert!(parsed.err.contains("rows have none"), "{}", parsed.err);
}

#[test]
fn the_help_of_a_leaf_whose_document_declares_a_long_note_says_that_long_prints_it() {
    let short = parse(&["du", "noted", "-h"]);
    let long = parse(&["du", "noted", "--help"]);

    let line = format!("\n{LONG_NOTE_LINE}\n");
    assert!(short.out.contains(&line), "{}", short.out);
    assert!(long.out.contains(&line), "{}", long.out);
    assert!(long.out.contains("  --long\n"), "{}", long.out);
}

#[test]
fn the_help_of_a_leaf_without_a_long_note_has_no_long_note_line() {
    let parsed = parse(&["du", "plain", "--help"]);

    assert!(!parsed.out.contains("Long note:"), "{}", parsed.out);
}
