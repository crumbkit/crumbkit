//! Tests for rows that print their own text line in place of the table
//! (ADR-0064).

use std::process::ExitCode;

use schemars::JsonSchema;
use serde::Serialize;
use serde_json::Value;

use super::{finish_output, finish_with};
use crate::help::Guide;
use crate::{Document, Listing, Row};

/// A link from a pointer to its target. A link without a target has no
/// text line of its own.
#[derive(Serialize, JsonSchema)]
struct Link {
    /// The name that points.
    pointer: String,
    /// What the pointer points at, when it points at anything.
    target: Option<String>,
    /// Whether the target exists.
    status: &'static str,
}

impl Row for Link {
    const COLUMNS: &'static [&'static str] = &["pointer", "target"];
    const LONG_COLUMNS: &'static [&'static str] = &["status"];
    const LINE: Option<&'static str> = Some("<POINTER> -> <TARGET>");

    fn text_line(&self) -> Option<String> {
        let target = self.target.as_ref()?;
        Some(format!("{} -> {target}", self.pointer))
    }
}

const LINKS: Document<Vec<Link>> = Document::new("links", "links", 1, "The test links.");
const LINK_LISTING: Document<Listing<Link>> =
    Document::new("links", "link-listing", 1, "The test links.");

fn link(pointer: &str, target: &str) -> Link {
    Link {
        pointer: pointer.to_owned(),
        target: Some(target.to_owned()),
        status: "ok",
    }
}

#[test]
fn a_row_with_its_own_text_line_prints_that_line_for_each_row_and_no_header() {
    let finished = finish_with(&[], LINKS, vec![link("a", "b"), link("c", "d")]);

    assert_eq!(finished.code, ExitCode::SUCCESS, "{}", finished.err);
    assert_eq!(finished.out, "a -> b\nc -> d\n");
    assert_eq!(finished.err, "");
}

#[test]
fn a_text_line_with_a_control_character_prints_it_escaped_on_one_line() {
    let finished = finish_with(&[], LINKS, vec![link("a\nb", "c\td")]);

    assert_eq!(finished.out, "a\\nb -> c\\td\n");
}

#[test]
fn a_row_without_a_text_line_prints_as_one_line_of_compact_json_with_every_field() {
    let unlinked = Link {
        pointer: "e".to_owned(),
        target: None,
        status: "broken",
    };
    let finished = finish_with(&[], LINKS, vec![link("a", "b"), unlinked]);

    assert_eq!(
        finished.out,
        "a -> b\n{\"pointer\":\"e\",\"target\":null,\"status\":\"broken\"}\n"
    );
}

#[test]
fn an_empty_result_of_rows_with_their_own_text_line_prints_nothing_on_standard_output() {
    let finished = finish_with(&[], LINKS, Vec::new());

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(finished.out, "");
}

#[test]
fn long_prints_crumbkit_s_table_for_rows_with_their_own_text_line() {
    let finished = finish_with(&["--long"], LINKS, vec![link("a", "b")]);

    assert_eq!(
        finished.out,
        "POINTER  TARGET  STATUS\na        b       ok\n"
    );
}

#[test]
fn fields_prints_crumbkit_s_table_for_rows_with_their_own_text_line() {
    let finished = finish_with(&["--fields", "target"], LINKS, vec![link("a", "b")]);

    assert_eq!(finished.out, "TARGET\nb\n");
}

#[test]
fn json_prints_the_compact_columns_of_rows_with_their_own_text_line() {
    let finished = finish_with(&["--json"], LINKS, vec![link("a", "b")]);

    let document: Value = serde_json::from_str(&finished.out).unwrap();
    assert_eq!(
        document["data"],
        serde_json::json!([{"pointer": "a", "target": "b"}])
    );
}

#[test]
fn a_listing_of_rows_with_their_own_text_line_keeps_its_footer_on_standard_error() {
    let listing = Listing::new(vec![link("a", "b")], 3).described("links");
    let finished = finish_with(&[], LINK_LISTING, listing);

    assert_eq!(finished.out, "a -> b\n");
    assert!(finished.err.contains("1 of 3"), "{}", finished.err);
}

#[test]
fn a_listing_of_rows_with_their_own_text_line_over_the_budget_prints_the_lines_that_fit() {
    let guide = Guide::new().output_budget(1024, "the tests print little");
    let mut output = super::built_with(
        <super::Tool as clap::CommandFactory>::command(),
        &guide,
        &["tool", "list"],
    );
    // Each line is 1,000 bytes, so only the first fits in 1,024 bytes.
    let pointer = "x".repeat(990);
    let rows = vec![link(&pointer, "b"), link(&pointer, "c")];
    let finished = finish_output(&mut output, LINK_LISTING, Listing::new(rows, 2));

    assert_eq!(finished.code, ExitCode::SUCCESS, "{}", finished.err);
    assert_eq!(finished.out, format!("{pointer} -> b\n"));
    assert!(finished.err.contains("1 of 2"), "{}", finished.err);
}

/// A setting that serializes as an array of its key and value, so it has
/// no columns, prints whole, and has its own text line when it has a value.
#[derive(Serialize, JsonSchema)]
struct Setting(String, Option<String>);

impl Row for Setting {
    const COLUMNS: &'static [&'static str] = &[];
    const WHOLE: bool = true;
    const LINE: Option<&'static str> = Some("<KEY> = <VALUE>");

    fn text_line(&self) -> Option<String> {
        let value = self.1.as_ref()?;
        Some(format!("{} = {value}", self.0))
    }
}

const SETTINGS: Document<Vec<Setting>> =
    Document::new("links", "settings", 1, "The test settings.");

fn setting(key: &str, value: Option<&str>) -> Setting {
    Setting(key.to_owned(), value.map(str::to_owned))
}

#[test]
fn a_whole_row_without_columns_that_declares_a_line_prints_its_line_and_json_only_without_one() {
    let rows = vec![setting("a", Some("b")), setting("c", None)];
    let finished = finish_with(&[], SETTINGS, rows);

    assert_eq!(finished.code, ExitCode::SUCCESS, "{}", finished.err);
    assert_eq!(finished.out, "a = b\n[\"c\",null]\n");
}

#[test]
fn json_prints_every_field_of_a_whole_row_that_declares_a_line() {
    let finished = finish_with(&["--json"], SETTINGS, vec![setting("a", Some("b"))]);

    let document: Value = serde_json::from_str(&finished.out).unwrap();
    assert_eq!(document["data"], serde_json::json!([["a", "b"]]));
}
