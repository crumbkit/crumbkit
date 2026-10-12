//! Tests for how `finish_to` prints a result within the output budget
//! (ADR-0046).

use std::fmt;
use std::process::ExitCode;

use clap::{CommandFactory as _, FromArgMatches as _};
use proptest::prelude::*;
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Value, json};

use super::{Finished, Tool, finish_output};
use crate::help::Guide;
use crate::twins::assert_one_word_apart;
use crate::{Batch, Code, Document, Error, Failure, Listing, Output, Parsed, Prose, Recovery, Row};

/// The smallest budget that a tool can set, which the tests use so that a
/// few rows pass it.
const BUDGET: usize = 1024;

const STARS: Document<Vec<Star>> = Document::new("sky", "stars", 1, "The test rows.");
const STAR: Document<Star> = Document::new("sky", "star", 1, "The test rows.");
const STAR_LISTING: Document<Listing<Star>> =
    Document::new("sky", "star-listing", 1, "The test rows.");
const STAR_BATCH: Document<Batch<Star, Error>> =
    Document::new("sky", "star-batch", 1, "The test rows.");
const CHART: Document<Prose<String, Error>> = Document::new("sky", "chart", 1, "The test text.");

const TOO_MUCH: Code = Code::new("sky", "stars", "too_much");

/// A row whose `story` column is long, so a few rows pass the budget.
#[derive(Serialize, JsonSchema)]
struct Star {
    name: String,
    story: String,
}

impl Row for Star {
    const COLUMNS: &'static [&'static str] = &["name", "story"];
}

/// Returns the star numbered `number`, whose row is about 100 bytes wide.
fn star(number: usize) -> Star {
    Star {
        name: format!("star-{number:02}"),
        story: "a".repeat(90),
    }
}

/// Returns the first `count` stars.
fn stars(count: usize) -> Vec<Star> {
    (1..=count).map(star).collect()
}

/// Returns the output flags for `flags` with a guide whose budget is
/// `budget`, as a tool's `main` sets them.
fn budgeted(budget: usize, flags: &[&str]) -> Output {
    let mut command = Tool::command();
    let args = ["tool", "list"].iter().chain(flags);
    let matches = command.try_get_matches_from_mut(args).unwrap();
    let mut output = Output::from_arg_matches(&matches).unwrap();
    let guide = Guide::new().output_budget(budget, "the tests print little");
    output.set_command(Parsed::guided(command, &guide));
    output
}

/// Finishes `value` with `flags` and a budget of [`BUDGET`].
fn finish_small<T: crate::Rows>(flags: &[&str], document: Document<T>, value: T) -> Finished {
    finish_output(&mut budgeted(BUDGET, flags), document, value)
}

fn parsed(out: &str) -> Value {
    serde_json::from_str(out).unwrap()
}

#[test]
fn a_listing_over_the_budget_stops_before_the_row_that_would_pass_it_and_exits_0() {
    let listing = Listing::new(stars(20), 20).described("stars");
    let finished = finish_small(&[], STAR_LISTING, listing);

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert!(finished.out.len() <= BUDGET, "{}", finished.out.len());
    let shown = finished.out.lines().count() - 2;
    assert!(shown > 0 && shown < 20, "{shown}");
    assert!(finished.out.ends_with(&format!(
        "Showing {shown} of 20 stars, because output stops at 1024 bytes. \
         Ask for less with --limit or --fields.\n"
    )));
    // Each row's line is 100 bytes, so one more row passes the budget.
    assert!(finished.out.len() + 100 > BUDGET);
    assert_eq!(finished.err, "");
}

#[test]
fn a_cut_listing_json_document_holds_cut_with_the_code_and_the_budget_beside_total() {
    let finished = finish_small(&["--json"], STAR_LISTING, Listing::new(stars(20), 20));

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert!(finished.out.len() <= BUDGET);
    let document = parsed(&finished.out);
    assert_eq!(document["total"], 20);
    assert_eq!(
        document["cut"],
        json!({"code": "crumbkit::output::over_budget", "budget": 1024})
    );
    let shown = document["data"].as_array().unwrap().len();
    assert!(shown > 0 && shown < 20, "{shown}");
}

#[test]
fn a_listing_that_fits_has_no_cut() {
    let finished = finish_small(&["--json"], STAR_LISTING, Listing::new(stars(2), 2));

    assert_eq!(parsed(&finished.out).get("cut"), None);
}

#[test]
fn a_list_over_the_budget_is_cut_as_a_listing_is_and_its_line_names_only_fields() {
    let finished = finish_small(&[], STARS, stars(20));

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert!(finished.out.len() <= BUDGET);
    assert!(
        finished
            .out
            .ends_with(", because output stops at 1024 bytes. Ask for less with --fields.\n"),
        "{}",
        finished.out
    );
}

#[test]
fn a_full_json_listing_over_the_budget_is_cut_too() {
    let finished = finish_small(
        &["--json", "--full"],
        STAR_LISTING,
        Listing::new(stars(20), 20),
    );

    assert!(finished.out.len() <= BUDGET);
    assert!(parsed(&finished.out)["cut"].is_object());
}

#[test]
fn a_document_with_its_own_over_budget_code_marks_its_cut_with_that_code() {
    let document = STAR_LISTING.over_budget(&TOO_MUCH);
    let finished = finish_small(&["--json"], document, Listing::new(stars(20), 20));

    assert_eq!(parsed(&finished.out)["cut"]["code"], "sky::stars::too_much");
}

#[test]
fn an_unwrapped_listing_cut_short_prints_its_line_on_standard_error() {
    let document = STAR_LISTING.without_envelope();
    let finished = finish_small(&["--json"], document, Listing::new(stars(20), 20));

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert!(finished.out.len() <= BUDGET);
    let shown = parsed(&finished.out).as_array().unwrap().len();
    assert_eq!(
        finished.err,
        format!(
            "Showing {shown} of 20, because output stops at 1024 bytes. \
             Ask for less with --limit or --fields.\n"
        )
    );
}

#[test]
fn a_batch_over_the_budget_answers_its_first_ids_and_fails_the_rest_with_one_failure_and_exits_1() {
    let batch = Batch::new(stars(20), Vec::new());
    let finished = finish_small(&["--json"], STAR_BATCH, batch);

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert!(finished.out.len() <= BUDGET);
    let document = parsed(&finished.out);
    let shown = document["data"].as_array().unwrap().len();
    assert!(shown > 0 && shown < 20, "{shown}");
    let failures = document["failures"].as_array().unwrap();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0]["code"], "crumbkit::output::over_budget");
    assert_eq!(
        failures[0]["fields"],
        json!({"left_out": 20 - shown, "budget": 1024})
    );
    assert_eq!(
        failures[0]["recovery"],
        json!({"kind": "hint", "text": "run the command again with the ids that were not answered"})
    );
}

#[test]
fn a_batch_over_the_budget_in_text_prints_the_failure_of_the_ids_left_out_on_standard_error() {
    let batch = Batch::new(stars(20), Vec::new());
    let finished = finish_small(&[], STAR_BATCH, batch);

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert!(finished.out.len() <= BUDGET);
    let shown = finished.out.lines().count() - 1;
    assert!(finished.err.starts_with(&format!(
        "error: {} more rows did not fit in the output budget of 1024 bytes\n",
        20 - shown
    )));
    assert!(
        finished.err.contains("crumbkit::output::over_budget"),
        "{}",
        finished.err
    );
}

#[test]
fn a_batch_always_answers_its_first_id_even_when_it_passes_the_budget() {
    let mut first = star(1);
    first.story = "a".repeat(2000);
    let batch = Batch::new(vec![first, star(2)], Vec::new());
    let finished = finish_small(&["--json"], STAR_BATCH, batch);

    assert_eq!(finished.code, ExitCode::FAILURE);
    let document = parsed(&finished.out);
    assert_eq!(document["data"].as_array().unwrap().len(), 1);
    assert_eq!(document["failures"][0]["fields"]["left_out"], 1);
}

#[test]
fn a_single_row_over_the_budget_is_refused_with_the_over_budget_code_and_exits_1() {
    let refused = ["--json"];
    let admitted = ["--json", "--fields=name"];
    assert_one_word_apart(&refused, &admitted);
    let mut big = star(1);
    big.story = "a".repeat(2000);

    let finished = finish_small(&refused, STAR, big);

    assert_eq!(finished.code, ExitCode::FAILURE);
    let document = parsed(&finished.out);
    assert_eq!(document["schema"], "crumbkit/failure/v1");
    assert_eq!(document["code"], "crumbkit::output::over_budget");
    assert_eq!(
        document["message"],
        "the result is larger than the output budget of 1024 bytes"
    );
    assert_eq!(
        document["recovery"],
        json!({"kind": "hint", "text": "ask for fewer fields with --fields"})
    );

    let mut big = star(1);
    big.story = "a".repeat(2000);
    let finished = finish_small(&admitted, STAR, big);
    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(
        finished.out,
        "{\"schema\":\"sky/star/v1\",\"data\":{\"name\":\"star-01\"}}\n"
    );
}

#[test]
fn a_document_that_always_prints_its_first_answer_prints_a_single_row_past_the_budget() {
    let mut big = star(1);
    big.story = "a".repeat(2000);
    let finished = finish_small(&["--json"], STAR.always_print_first(), big);

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert!(finished.out.len() > BUDGET);
    assert_eq!(parsed(&finished.out)["schema"], "sky/star/v1");
}

#[test]
fn a_prose_result_over_the_budget_is_refused_and_no_flag_makes_it_smaller() {
    let finished = finish_small(&[], CHART, Prose::new("b".repeat(2000)));

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert_eq!(finished.out, "");
    assert!(
        finished
            .err
            .starts_with("error: the result is larger than the output budget of 1024 bytes\n")
    );
    assert!(
        finished
            .err
            .contains("no next step: no flag makes this answer smaller\n"),
        "{}",
        finished.err
    );
}

#[test]
fn budget_reports_the_bytes_left_after_finish_prints_a_result() {
    let mut output = budgeted(BUDGET, &[]);
    assert_eq!(output.budget(), BUDGET);

    let finished = finish_output(&mut output, STARS, stars(1));

    assert_eq!(output.budget(), BUDGET - finished.out.len());
}

#[test]
fn an_output_from_a_guide_without_a_budget_has_crumbkit_s_default_of_one_mebibyte() {
    let mut output = Output::default();
    output.set_command(Parsed::guided(Tool::command(), &Guide::new()));

    assert_eq!(output.budget(), Output::DEFAULT_BUDGET);
    assert_eq!(Output::DEFAULT_BUDGET, 1_048_576);
}

#[test]
#[should_panic(expected = "an output budget is at least 1024 bytes")]
fn a_guide_refuses_an_output_budget_below_1024_bytes() {
    let _ = Guide::new().output_budget(1023, "too small");
}

/// A failure of a batch, so the property covers the batch's own failures.
#[derive(Debug)]
struct Dim(usize);

impl fmt::Display for Dim {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "star {} is too dim to see", self.0)
    }
}

impl std::error::Error for Dim {}

impl Failure for Dim {
    fn code(&self) -> &'static Code {
        &TOO_MUCH
    }

    fn recovery(&self) -> Recovery {
        Recovery::Retry
    }
}

const DIM_BATCH: Document<Batch<Star, Dim>> =
    Document::new("sky", "dim-batch", 1, "The test rows.");

proptest! {
    #[test]
    fn standard_output_never_passes_the_budget(
        lengths in proptest::collection::vec(0_usize..400, 1..40),
        budget in 1024_usize..4096,
        json in any::<bool>(),
        dim in 0_usize..3,
    ) {
        let rows = || {
            lengths
                .iter()
                .enumerate()
                .map(|(number, &length)| Star {
                    name: format!("star-{number}"),
                    story: "c".repeat(length),
                })
                .collect::<Vec<_>>()
        };
        let flags: &[&str] = if json { &["--json"] } else { &[] };
        let total = lengths.len();

        let listing = finish_output(
            &mut budgeted(budget, flags),
            STAR_LISTING,
            Listing::new(rows(), total).described("stars"),
        );
        prop_assert!(listing.out.len() <= budget);

        let failures = (0..dim).map(Dim).collect();
        let batch = finish_output(
            &mut budgeted(budget, flags),
            DIM_BATCH,
            Batch::new(rows(), failures),
        );
        // The first row always prints, so only a batch whose first row fits
        // stays within the budget.
        let first = finish_output(
            &mut budgeted(budget, flags),
            DIM_BATCH,
            Batch::new(rows().into_iter().take(1).collect(), (0..dim).map(Dim).collect()),
        );
        if first.out.len() + 300 <= budget {
            prop_assert!(batch.out.len() <= budget);
        }
    }
}

#[test]
fn a_listing_cut_to_no_rows_prints_the_cut_line_and_no_empty_note() {
    let big = Star {
        name: "big".to_owned(),
        story: "a".repeat(BUDGET),
    };
    let listing = Listing::new(vec![big], 1).described("stars");

    let finished = finish_small(&[], STAR_LISTING, listing);

    assert!(
        finished.out.ends_with(
            "Showing 0 of 1 stars, because output stops at 1024 bytes. \
             Ask for less with --limit or --fields.\n"
        ),
        "{}",
        finished.out
    );
    assert!(!finished.out.contains("note:"), "{}", finished.out);
}
