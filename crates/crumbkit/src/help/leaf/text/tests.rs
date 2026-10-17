//! Tests for the `Text:` line of a leaf whose rows print their own text
//! line (ADR-0064).

use std::process::ExitCode;

use clap::{Arg, ArgAction, Command};
use schemars::JsonSchema;
use serde::Serialize;

use crate::help::{Effects, Example, Guide, Leaf, STREAM_LINE, WHOLE_LINE};
use crate::{Document, Row, Sink, Spelling, Stream, parse_to};

/// A tool with a global `--json` and one leaf, `tool links check`.
fn tool() -> Command {
    Command::new("tool")
        .subcommand_required(true)
        .arg(
            Arg::new("json")
                .long("json")
                .global(true)
                .action(ArgAction::SetTrue)
                .help("Print JSON"),
        )
        .subcommand(
            Command::new("links").subcommand(Command::new("check").about("Check the links")),
        )
}

/// Runs `parse_to` over `tool()` with `guide` for `args`, which ask for
/// help, and returns the page it printed.
fn help(guide: &Guide, args: &[&str]) -> String {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result = parse_to::<(), _, _, _, _>(
        &mut Sink::new(&mut out, &mut err),
        tool(),
        guide,
        Spelling::COLONS,
        args,
    );
    assert_eq!(result.err().map(ExitCode::from), Some(ExitCode::SUCCESS));
    String::from_utf8(out).unwrap()
}

/// A link that prints its own text line and has a long column.
#[derive(Serialize, JsonSchema)]
struct Link {
    /// The name that points.
    pointer: String,
    /// What the pointer points at.
    target: String,
    /// Whether the target exists.
    status: String,
}

impl Row for Link {
    const COLUMNS: &'static [&'static str] = &["pointer", "target"];
    const LONG_COLUMNS: &'static [&'static str] = &["status"];
    const LINE: Option<&'static str> = Some("<POINTER> -> <TARGET>");
}

/// A link that prints its own text line and has no long column.
#[derive(Serialize, JsonSchema)]
struct Short {
    /// The name that points.
    pointer: String,
}

impl Row for Short {
    const COLUMNS: &'static [&'static str] = &["pointer"];
    const LINE: Option<&'static str> = Some("<POINTER>");
}

/// Returns the guide whose one leaf, `links check`, declares `leaf`'s
/// document.
const fn guide(leaf: &'static [Leaf]) -> Guide {
    Guide::new().leaves(leaf)
}

const LINKS: Document<Vec<Link>> = Document::new("tool", "links-check", 1, "The links.");
const LINK_STREAM: Document<Stream<Link>> =
    Document::new("tool", "links-check", 1, "Each link as it is checked.");
const SHORTS: Document<Vec<Short>> = Document::new("tool", "links-check", 1, "The links.");

const LINKS_LEAF: &[Leaf] = &[Leaf::new(&["links", "check"])
    .examples(&[Example::new(&[], "Check every link")])
    .effects(Effects::read_only())
    .document(LINKS)];
const STREAM_LEAF: &[Leaf] = &[Leaf::new(&["links", "check"])
    .examples(&[Example::new(&[], "Check every link")])
    .effects(Effects::read_only())
    .stream(LINK_STREAM)];
const SHORT_LEAF: &[Leaf] = &[Leaf::new(&["links", "check"])
    .examples(&[Example::new(&[], "Check every link")])
    .effects(Effects::read_only())
    .document(SHORTS)];

#[test]
fn a_leaf_holds_the_text_line_form_of_its_row_type() {
    assert_eq!(LINKS_LEAF[0].line(), Some("<POINTER> -> <TARGET>"));
    assert_eq!(Leaf::new(&["links", "check"]).line(), None);
}

#[test]
fn the_help_of_a_leaf_whose_rows_print_their_own_line_names_the_line_after_the_effects() {
    let short = help(&guide(LINKS_LEAF), &["tool", "links", "check", "-h"]);
    let long = help(&guide(LINKS_LEAF), &["tool", "links", "check", "--help"]);

    let expected = "Effects: read-only, closed-world.\nText: one line per row, <POINTER> -> <TARGET>; --fields and --long print a table.\n";
    assert!(short.contains(expected), "{short}");
    assert!(long.contains(expected), "{long}");
    assert!(long.contains("Columns:"), "{long}");
}

#[test]
fn the_text_line_of_a_stream_follows_its_output_line() {
    let page = help(&guide(STREAM_LEAF), &["tool", "links", "check", "-h"]);

    let expected = format!(
        "{STREAM_LINE}\nText: one line per row, <POINTER> -> <TARGET>; --fields and --long print a table.\n"
    );
    assert!(page.contains(&expected), "{page}");
}

#[test]
fn the_text_line_of_rows_without_long_columns_names_only_fields() {
    let page = help(&guide(SHORT_LEAF), &["tool", "links", "check", "-h"]);

    assert!(
        page.contains("Text: one line per row, <POINTER>; --fields prints a table.\n"),
        "{page}"
    );
}

#[test]
fn the_help_of_a_leaf_whose_rows_print_a_table_has_no_text_line() {
    const PLAIN: &[Leaf] = &[Leaf::new(&["links", "check"])
        .examples(&[Example::new(&[], "Check every link")])
        .effects(Effects::read_only())];
    let page = help(&guide(PLAIN), &["tool", "links", "check", "--help"]);

    assert!(!page.contains("Text:"), "{page}");
}

/// A setting that serializes as an array of its key and value, so it has
/// no columns, prints whole, and has its own text line.
#[derive(Serialize, JsonSchema)]
struct Setting(String, String);

impl Row for Setting {
    const COLUMNS: &'static [&'static str] = &[];
    const WHOLE: bool = true;
    const LINE: Option<&'static str> = Some("<KEY> = <VALUE>");
}

#[test]
fn the_text_line_of_whole_rows_without_columns_names_no_table_and_replaces_the_json_output_line() {
    const SETTINGS: Document<Vec<Setting>> =
        Document::new("tool", "links-check", 1, "The settings.");
    const SETTINGS_LEAF: &[Leaf] = &[Leaf::new(&["links", "check"])
        .examples(&[Example::new(&[], "Check every link")])
        .effects(Effects::read_only())
        .document(SETTINGS)];
    let page = help(&guide(SETTINGS_LEAF), &["tool", "links", "check", "-h"]);

    assert!(
        page.contains(
            "Effects: read-only, closed-world.\nText: one line per row, <KEY> = <VALUE>.\n"
        ),
        "{page}"
    );
    assert!(!page.contains(WHOLE_LINE), "{page}");
}
