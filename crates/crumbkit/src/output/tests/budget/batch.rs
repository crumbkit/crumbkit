//! Tests for how `finish_to` prints a batch within the output budget: the
//! rows that fit, then one failure that names the keys left out
//! (ADR-0046).

use std::process::ExitCode;

use serde_json::json;

use super::{BUDGET, STAR_BATCH, Star, finish_small, keyed, parsed, star, stars};
use crate::{Batch, Error};

/// Returns a batch of `tool show` that takes `keys` and holds `rows`, one
/// row for each of the first keys.
fn fetched(keys: &[String], rows: Vec<Star>) -> Batch<Star, Error> {
    let mut batch = Batch::new(&["show"], "names", keys);
    batch.extend(rows.into_iter().map(Ok));
    batch
}

/// Returns the names of the stars from `from` to `to`, as their keys.
fn names(from: usize, to: usize) -> Vec<String> {
    (from..=to).map(|number| star(number).name).collect()
}

#[test]
fn a_batch_over_the_budget_answers_its_first_keys_and_names_the_rest_in_one_failure_and_exits_1() {
    let finished = finish_small(
        &["--json"],
        STAR_BATCH,
        keyed(stars(20), Vec::<Error>::new()),
    );

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert!(finished.out.len() <= BUDGET);
    let document = parsed(&finished.out);
    let shown = document["data"].as_array().unwrap().len();
    assert!(shown > 0 && shown < 20, "{shown}");
    let failures = document["failures"].as_array().unwrap();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0]["code"], "crumbkit::output::over_budget");
    assert_eq!(
        failures[0]["message"],
        format!(
            "{} more answers did not fit in the output budget of 1024 bytes",
            20 - shown
        )
    );
    assert_eq!(
        failures[0]["fields"],
        json!({"left_out": names(shown + 1, 20), "budget": 1024})
    );
    assert_eq!(
        failures[0]["recovery"],
        json!({"kind": "run", "call": "tool show <NAME>", "each": "left_out"})
    );
}

#[test]
fn a_batch_over_the_budget_in_text_names_every_key_left_out_on_standard_error() {
    let finished = finish_small(&[], STAR_BATCH, keyed(stars(20), Vec::<Error>::new()));

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert!(finished.out.len() <= BUDGET);
    let shown = finished.out.lines().count() - 1;
    assert_eq!(
        finished.err,
        format!(
            "error: {} more answers did not fit in the output budget of 1024 bytes\n  \
             help:  run `tool show <NAME>` for each value of left_out: {}\n  \
             code:  crumbkit::output::over_budget\n",
            20 - shown,
            names(shown + 1, 20).join(", ")
        )
    );
}

#[test]
fn a_batch_whose_rows_fit_fails_with_the_keys_that_were_never_fetched() {
    let keys = names(1, 4);
    let finished = finish_small(&["--json"], STAR_BATCH, fetched(&keys, stars(2)));

    assert_eq!(finished.code, ExitCode::FAILURE);
    let document = parsed(&finished.out);
    assert_eq!(document["data"].as_array().unwrap().len(), 2);
    assert_eq!(
        document["failures"][0]["fields"],
        json!({"left_out": ["star-03", "star-04"], "budget": 1024})
    );
}

#[test]
fn a_key_left_out_quotes_for_a_shell_in_text_and_repeats_as_often_as_it_was_given() {
    let keys = ["star-01", "Le Guin", "Le Guin"].map(str::to_owned);
    let finished = finish_small(&[], STAR_BATCH, fetched(&keys, stars(1)));

    assert!(
        finished
            .err
            .contains("for each value of left_out: 'Le Guin', 'Le Guin'\n"),
        "{}",
        finished.err
    );
}

#[test]
fn a_row_pushed_after_the_last_key_counts_as_unlisted_when_it_is_left_out() {
    let keys = names(1, 1);
    let finished = finish_small(&["--json"], STAR_BATCH, fetched(&keys, stars(20)));

    let document = parsed(&finished.out);
    let shown = document["data"].as_array().unwrap().len();
    assert_eq!(
        document["failures"][0]["fields"],
        json!({"left_out": [], "unlisted": 20 - shown, "budget": 1024})
    );
}

#[test]
fn a_failure_too_long_for_the_budget_names_its_first_keys_and_counts_the_rest_as_unlisted() {
    let keys: Vec<String> = (0..100)
        .map(|number| format!("a-long-key-{number:03}-{}", "k".repeat(20)))
        .collect();
    let finished = finish_small(&["--json"], STAR_BATCH, fetched(&keys, stars(1)));

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert!(finished.out.len() <= BUDGET, "{}", finished.out.len());
    let document = parsed(&finished.out);
    assert_eq!(document["data"].as_array().unwrap().len(), 1);
    let fields = &document["failures"][0]["fields"];
    let listed = fields["left_out"].as_array().unwrap();
    assert_ne!(listed.len(), 0);
    assert_eq!(*listed, keys[1..=listed.len()]);
    assert_eq!(fields["unlisted"], 99 - listed.len());
    assert_eq!(
        document["failures"][0]["message"],
        "99 more answers did not fit in the output budget of 1024 bytes"
    );
}

#[test]
fn a_batch_always_answers_its_first_key_and_lists_no_key_when_that_answer_passes_the_budget() {
    let mut first = star(1);
    first.story = "a".repeat(2000);
    let finished = finish_small(
        &["--json"],
        STAR_BATCH,
        keyed(vec![first, star(2)], Vec::<Error>::new()),
    );

    assert_eq!(finished.code, ExitCode::FAILURE);
    let document = parsed(&finished.out);
    assert_eq!(document["data"].as_array().unwrap().len(), 1);
    assert_eq!(
        document["failures"][0]["fields"],
        json!({"left_out": [], "unlisted": 1, "budget": 1024})
    );
}
