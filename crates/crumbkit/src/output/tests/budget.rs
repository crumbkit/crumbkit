//! Tests for how `finish_to` prints a result within the output budget
//! (ADR-0046).

use std::fmt;
use std::process::ExitCode;

use clap::CommandFactory as _;
use proptest::prelude::*;
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Value, json};

use super::{Finished, Tool, finish_output};
use crate::Envelope;

mod batch;
mod rerun;
mod room;
use crate::help::{Exit, Guide, Rename};
use crate::twins::assert_one_word_apart;
use crate::{
    AsFailure, Batch, Code, Document, Error, Failure, JsonLayout, Listing, Output, Prose, Recovery,
    Row,
};

/// A budget that a few rows pass.
const BUDGET: usize = 1024;

const STARS: Document<Vec<Star>> = Document::new("sky", "stars", 1, "The test rows.");
const STAR: Document<Star> = Document::new("sky", "star", 1, "The test rows.");
const STAR_LISTING: Document<Listing<Star>> =
    Document::new("sky", "star-listing", 1, "The test rows.");
const STAR_BATCH: Document<Batch<Star, Error>> =
    Document::new("sky", "star-batch", 1, "The test rows.");
const CHART: Document<Prose<String, Error>> = Document::new("sky", "chart", 1, "The test text.");

const TOO_MUCH: Code = Code::new("sky", "stars", "too_much");

/// A guide with the budget of [`BUDGET`] that prints `TOO_MUCH` in place of
/// crumbkit's over-budget code and lists it under 9.
const RENAMED: Guide = Guide::new()
    .output_budget(BUDGET, "the tests print little")
    .codes(&[Rename::new(&crate::codes::OUTPUT_OVER_BUDGET, &TOO_MUCH)])
    .exits(&[Exit::new(9, "The answer was too long.").failures(&[&TOO_MUCH])]);

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

/// Returns a batch of `tool show` whose keys are the names of `rows`, then
/// one key for each failure, with every row and then every failure pushed
/// in that order.
fn keyed<E: AsFailure>(rows: Vec<Star>, failures: Vec<E>) -> Batch<Star, E> {
    let mut keys: Vec<String> = rows.iter().map(|row| row.name.clone()).collect();
    keys.extend((0..failures.len()).map(|number| format!("missing-{number}")));
    let mut batch = Batch::new(&["show"], "names", &keys);
    batch.extend(rows.into_iter().map(Ok));
    batch.extend(failures.into_iter().map(Err));
    batch
}

/// Returns the output flags for `flags` with a guide whose budget is
/// `budget`, as `Output::new` builds them.
fn budgeted(budget: usize, flags: &[&str]) -> Output {
    guided(
        &Guide::new().output_budget(budget, "the tests print little"),
        flags,
    )
}

/// Returns the output flags for `flags` with `guide`, as `Output::new`
/// builds them.
fn guided(guide: &Guide, flags: &[&str]) -> Output {
    let args: Vec<&str> = ["tool", "list"].iter().chain(flags).copied().collect();
    super::built_with(Tool::command(), guide, &args)
}

/// Finishes `value` with `flags` and a budget of [`BUDGET`].
fn finish_small<T: crate::Rows>(flags: &[&str], document: Document<T>, value: T) -> Finished {
    finish_output(&mut budgeted(BUDGET, flags), document, value)
}

fn parsed(out: &str) -> Value {
    serde_json::from_str(out).unwrap()
}

#[test]
fn a_pretty_layout_cuts_a_listing_where_its_indented_bytes_stop_fitting() {
    let listing = || Listing::new(stars(20), 20).described("stars");
    let guide = Guide::new()
        .output_budget(BUDGET, "the tests print little")
        .json_layout(JsonLayout::Pretty {
            reason: "the readers expect it",
        });
    let compact = finish_small(&["--json"], STAR_LISTING, listing());
    let indented = finish_output(&mut guided(&guide, &["--json"]), STAR_LISTING, listing());

    assert_eq!(indented.code, ExitCode::SUCCESS);
    assert!(indented.out.len() <= BUDGET, "{}", indented.out.len());
    assert!(
        indented.out.starts_with("{\n  \"schema\""),
        "{}",
        indented.out
    );
    let shown = |out: &str| parsed(out)["data"].as_array().unwrap().len();
    assert!(shown(&indented.out) < shown(&compact.out));
    assert_eq!(parsed(&indented.out)["cut"]["budget"], BUDGET);
}

#[test]
fn a_listing_over_the_budget_stops_before_the_row_that_would_pass_it_and_exits_0() {
    let listing = Listing::new(stars(20), 20).described("stars");
    let finished = finish_small(&[], STAR_LISTING, listing);

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert!(finished.out.len() <= BUDGET, "{}", finished.out.len());
    let shown = finished.out.lines().count() - 1;
    assert!(shown > 0 && shown < 20, "{shown}");
    assert_eq!(
        finished.err,
        format!(
            "Showing {shown} of 20 stars, because output stops at 1024 bytes. \
             Ask for less with --limit or --fields.\n"
        )
    );
    // Each row's line is 100 bytes, so one more row passes the budget.
    assert!(finished.out.len() + 100 > BUDGET);
}

#[test]
fn a_cut_listing_json_document_holds_cut_with_the_code_and_the_budget_beside_total_and_truncated() {
    let finished = finish_small(&["--json"], STAR_LISTING, Listing::new(stars(20), 20));

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert!(finished.out.len() <= BUDGET);
    let document = parsed(&finished.out);
    assert_eq!(document["total"], 20);
    assert_eq!(document["truncated"], true);
    assert_eq!(
        document["cut"],
        json!({"code": "tool::output::over_budget", "budget": 1024})
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
            .err
            .ends_with(", because output stops at 1024 bytes. Ask for less with --fields.\n"),
        "{}",
        finished.err
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
fn a_guide_that_renames_the_over_budget_code_marks_a_cut_with_the_tool_s_code() {
    let mut output = guided(&RENAMED, &["--json"]);
    let finished = finish_output(&mut output, STAR_LISTING, Listing::new(stars(20), 20));

    assert_eq!(parsed(&finished.out)["cut"]["code"], "sky::stars::too_much");
}

#[test]
fn a_row_refused_with_a_renamed_over_budget_code_exits_with_the_code_the_guide_lists() {
    let star = Star {
        name: "vega".to_owned(),
        story: "a".repeat(2 * BUDGET),
    };
    let mut output = guided(&RENAMED, &["--json"]);
    let finished = finish_output(&mut output, STAR, star);

    assert_eq!(parsed(&finished.out)["code"], "sky::stars::too_much");
    assert_eq!(finished.code, ExitCode::from(9));
}

#[test]
fn an_unwrapped_listing_cut_short_prints_its_line_on_standard_error() {
    let document = STAR_LISTING.envelope(Envelope::Without);
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
fn a_single_row_over_the_budget_is_refused_with_the_over_budget_code_and_exits_1() {
    let refused = ["--json"];
    let admitted = ["--json", "--fields=name"];
    assert_one_word_apart(&refused, &admitted);
    let mut big = star(1);
    big.story = "a".repeat(2000);

    let finished = finish_small(&refused, STAR, big);

    assert_eq!(finished.code, ExitCode::FAILURE);
    let document = parsed(&finished.out);
    assert_eq!(document["schema"], "tool/failure/v1");
    assert_eq!(document["code"], "tool::output::over_budget");
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
    let output = Output::new(
        crate::ViewFlags::new(),
        Tool::command(),
        &Guide::new(),
        ["tool"],
    )
    .unwrap();

    assert_eq!(output.budget(), Output::DEFAULT_BUDGET);
    assert_eq!(Output::DEFAULT_BUDGET, 1_048_576);
}

#[test]
fn a_guide_can_set_an_output_budget_below_one_kibibyte_and_a_listing_fits_in_it() {
    let finished = finish_output(
        &mut budgeted(300, &["--json"]),
        STAR_LISTING,
        Listing::new(stars(5), 5).described("stars"),
    );

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert!(finished.out.len() <= 300, "{}", finished.out.len());
    assert_eq!(parsed(&finished.out)["cut"]["budget"], 300);
}

#[test]
fn a_run_with_a_guide_budget_of_1_byte_prints_the_over_budget_failure() {
    let finished = finish_output(&mut budgeted(1, &["--json"]), STAR, star(1));

    assert_eq!(finished.code, ExitCode::FAILURE);
    let document = parsed(&finished.out);
    assert_eq!(document["code"], "tool::output::over_budget");
    assert_eq!(document["fields"], json!({"budget": 1}));
}

#[test]
fn a_run_with_a_guide_budget_of_0_bytes_refuses_a_listing_with_the_over_budget_failure() {
    let listing = Listing::new(stars(5), 5).described("stars");
    let finished = finish_output(&mut budgeted(0, &["--json"]), STAR_LISTING, listing);

    assert_eq!(finished.code, ExitCode::FAILURE);
    let document = parsed(&finished.out);
    assert_eq!(document["code"], "tool::output::over_budget");
    assert_eq!(document["fields"], json!({"budget": 0}));
}

#[test]
fn a_batch_with_a_guide_budget_of_0_bytes_refuses_its_first_answer_and_names_the_rest_in_the_over_budget_failure()
 {
    let batch = keyed::<Error>(stars(3), vec![]);
    let finished = finish_output(&mut budgeted(0, &["--json"]), STAR_BATCH, batch);

    assert_eq!(finished.code, ExitCode::FAILURE);
    let document = parsed(&finished.out);
    assert_eq!(document["data"], json!([]));
    assert_eq!(document["failures"][0]["key"], "star-01");
    assert_eq!(document["failures"][1]["code"], "tool::output::over_budget");
    assert_eq!(
        document["failures"][1]["fields"]["left_out"],
        json!(["star-02", "star-03"])
    );
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
            keyed(rows(), failures),
        );
        // A first row that fits alone prints even when the failure beside it
        // passes the budget, so only a batch whose first row fits with room
        // for that failure stays within the budget.
        let first = finish_output(
            &mut budgeted(budget, flags),
            DIM_BATCH,
            keyed(rows().into_iter().take(1).collect(), (0..dim).map(Dim).collect()),
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

    assert_eq!(finished.out, "NAME  STORY\n");
    assert_eq!(
        finished.err,
        "Showing 0 of 1 stars, because output stops at 1024 bytes. \
         Ask for less with --limit or --fields.\n"
    );
}

#[test]
fn the_json_of_an_uncounted_listing_that_the_budget_cut_has_truncated_and_cut_and_no_total() {
    for more in [false, true] {
        let listing = Listing::uncounted(stars(20), more).described("stars");

        let finished = finish_small(&["--json"], STAR_LISTING, listing);

        let document = parsed(&finished.out);
        assert_eq!(document.get("total"), None, "more: {more}");
        assert_eq!(document["truncated"], true, "more: {more}");
        assert_eq!(document["cut"]["budget"], BUDGET, "more: {more}");
    }
}

#[test]
fn the_cut_line_of_an_uncounted_listing_that_holds_every_row_gives_no_count() {
    let listing = Listing::uncounted(stars(20), false).described("stars");

    let finished = finish_small(&[], STAR_LISTING, listing);

    let shown = finished.out.lines().count() - 1;
    assert_eq!(
        finished.err,
        format!(
            "Showing {shown} stars, because output stops at 1024 bytes. \
             Ask for less with --limit or --fields.\n"
        )
    );
}

#[test]
fn the_cut_line_of_a_listing_without_a_total_gives_no_count() {
    let listing = Listing::uncounted(stars(20), true).described("stars");

    let finished = finish_small(&[], STAR_LISTING, listing);

    let shown = finished.out.lines().count() - 1;
    assert_eq!(
        finished.err,
        format!(
            "Showing {shown} stars, because output stops at 1024 bytes. \
             Ask for less with --limit or --fields.\n"
        )
    );
}
