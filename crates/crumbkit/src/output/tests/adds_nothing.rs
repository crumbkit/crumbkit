//! Tests for the note that `finish_to` prints when `--long` or `--full`
//! adds nothing to the JSON, and for where the note sits among the other
//! lines on standard error (ADR-0149).

use std::fmt;
use std::process::ExitCode;

use clap::CommandFactory as _;
use schemars::JsonSchema;
use serde::Serialize;

use super::{Finished, PLANETS, Planet, Tool, finish_output, finish_with, mercury, saturn};
use crate::answered::answered;
use crate::help::Guide;
use crate::{Batch, Code, Document, Envelope, Error, Failure, Listing, Recovery, Row};

/// The note that `--long` adds nothing to the JSON.
const LONG_NOTE: &str =
    "note: --long adds nothing to --json here; the JSON already holds the whole result.\n";
/// The note that `--full` adds nothing to the JSON.
const FULL_NOTE: &str =
    "note: --full adds nothing to --json here; the JSON already holds the whole result.\n";

const DRY_RUN_LINE: &str = "This was a dry run, so nothing changed.\n";

const GONE: Code = Code::new("solar", "planets", "gone");

/// A failure of a planet that is gone.
#[derive(Debug)]
struct Gone;

impl fmt::Display for Gone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Vulcan is gone")
    }
}

impl std::error::Error for Gone {}

impl Failure for Gone {
    fn code(&self) -> &'static Code {
        &GONE
    }

    fn recovery(&self) -> Recovery {
        Recovery::Retry
    }
}

/// A comet whose JSON readers expect every field.
#[derive(Serialize, JsonSchema)]
struct Comet {
    /// The comet's name.
    name: &'static str,
    /// The comet's period in years.
    period: u32,
}

impl Row for Comet {
    const COLUMNS: &'static [&'static str] = &["name"];
    const WHOLE: bool = true;
}

/// Finishes `value` as `document` with `flags` on a dry run.
fn finish_dry<T: crate::Rows>(flags: &[&str], document: Document<T>, value: T) -> Finished {
    let mut output = super::parse_against(Tool::command(), flags);
    output.dry_run = true;
    finish_output(output, document, value)
}

#[test]
fn long_and_full_with_json_on_an_unwrapped_document_print_the_json_and_a_note() {
    let unwrapped = PLANETS.envelope(Envelope::Without);
    let json = finish_with(&["--json"], unwrapped, vec![mercury(), saturn()]);

    let long = finish_with(&["--json", "--long"], unwrapped, vec![mercury(), saturn()]);
    let full = finish_with(&["--json", "--full"], unwrapped, vec![mercury(), saturn()]);

    assert_eq!(json.err, "");
    assert_eq!(long.code, ExitCode::SUCCESS);
    assert_eq!(long.out, json.out);
    assert_eq!(long.err, LONG_NOTE);
    assert_eq!(full.code, ExitCode::SUCCESS);
    assert_eq!(full.out, json.out);
    assert_eq!(full.err, FULL_NOTE);
}

#[test]
fn long_with_json_on_unwrapped_rows_with_long_columns_prints_a_note() {
    // A planet has the long column `ringed`, so `--long` is offered and
    // changes the text, and the unwrapped JSON already holds every field.
    let unwrapped = PLANETS.envelope(Envelope::Without);
    let text = finish_with(&["--long"], unwrapped, vec![saturn()]);

    let long = finish_with(&["--json", "--long"], unwrapped, vec![saturn()]);

    assert_eq!(text.err, "");
    assert!(text.out.contains("RINGED"), "{}", text.out);
    assert_eq!(long.code, ExitCode::SUCCESS);
    assert_eq!(
        long.out,
        "[{\"name\":\"Saturn\",\"moons\":274,\"ringed\":true,\"au\":9.54}]\n"
    );
    assert_eq!(long.err, LONG_NOTE);
}

#[test]
fn the_note_prints_before_the_dry_run_line() {
    let unwrapped = PLANETS.envelope(Envelope::Without);

    let finished = finish_dry(&["--json", "--full"], unwrapped, vec![saturn()]);

    assert_eq!(finished.err, format!("{FULL_NOTE}{DRY_RUN_LINE}"));
}

#[test]
fn the_note_prints_before_the_failures_of_an_unwrapped_batch_and_the_dry_run_line() {
    const SHOW: Document<Batch<Planet, Error>> =
        Document::new("solar", "planets-show", 1, "The test rows.").envelope(Envelope::Without);
    let json = finish_dry(
        &["--json"],
        SHOW,
        answered(vec![saturn()], vec![Gone.into()]),
    );

    let full = finish_dry(
        &["--json", "--full"],
        SHOW,
        answered(vec![saturn()], vec![Gone.into()]),
    );

    assert_eq!(full.code, ExitCode::FAILURE);
    assert_eq!(full.out, json.out);
    let failures = json.err.strip_suffix(DRY_RUN_LINE).unwrap();
    assert!(
        failures.starts_with("error: Vulcan is gone\n"),
        "{}",
        json.err
    );
    assert_eq!(full.err, format!("{FULL_NOTE}{failures}{DRY_RUN_LINE}"));
}

#[test]
fn the_note_is_the_only_line_on_standard_error_of_an_enveloped_batch_with_failures() {
    const COMETS: Document<Batch<Comet, Error>> =
        Document::new("solar", "comets-show", 1, "The test comets.");
    let halley = || Comet {
        name: "Halley",
        period: 76,
    };
    let json = finish_with(
        &["--json"],
        COMETS,
        answered(vec![halley()], vec![Gone.into()]),
    );

    let full = finish_with(
        &["--json", "--full"],
        COMETS,
        answered(vec![halley()], vec![Gone.into()]),
    );

    assert_eq!(full.code, ExitCode::FAILURE);
    assert_eq!(full.out, json.out);
    assert!(json.out.contains("Vulcan is gone"), "{}", json.out);
    assert_eq!(json.err, "");
    assert_eq!(full.err, FULL_NOTE);
}

#[test]
fn the_note_follows_the_cut_line_of_an_unwrapped_listing_cut_at_the_budget() {
    const LISTING: Document<Listing<Planet>> =
        Document::new("solar", "planet-listing", 1, "The test rows.").envelope(Envelope::Without);
    let guide = Guide::new().output_budget(200, "the tests print little");
    let planets = || Listing::new((0..20).map(|_| saturn()).collect(), 20).described("planets");
    let budgeted = |flags: &[&str]| {
        let args: Vec<&str> = ["tool", "list"].iter().chain(flags).copied().collect();
        finish_output(
            super::built_with(Tool::command(), &guide, &args),
            LISTING,
            planets(),
        )
    };
    let json = budgeted(&["--json"]);

    let full = budgeted(&["--json", "--full"]);

    assert_eq!(full.code, json.code);
    assert_eq!(full.out, json.out);
    assert!(!json.err.is_empty(), "the listing was not cut");
    assert_eq!(full.err, format!("{}{FULL_NOTE}", json.err));
}
