//! Tests for how `finish_to` prints a batch within the output budget: the
//! rows that fit, then one failure that names the keys left out
//! (ADR-0046).

use std::process::ExitCode;

use serde_json::json;

use super::{
    BUDGET, DIM_BATCH, Dim, STAR_BATCH, Star, budgeted, finish_small, keyed, parsed, star, stars,
};
use crate::output::tests::finish_output;
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
    assert_eq!(failures[0]["code"], "tool::output::over_budget");
    assert_eq!(
        failures[0]["message"],
        format!(
            "{} more answers did not fit in the output budget of 1024 bytes",
            20 - shown
        )
    );
    assert_eq!(
        failures[0]["fields"],
        json!({"left_out": names(shown + 1, 20), "left_out_from": shown, "budget": 1024})
    );
    assert_eq!(
        failures[0]["recovery"],
        json!({"kind": "run", "call": "tool show <NAME>...", "all": "left_out"})
    );
}

#[test]
fn a_batch_over_the_budget_in_text_names_every_key_left_out_in_one_call_on_standard_error() {
    let finished = finish_small(&[], STAR_BATCH, keyed(stars(20), Vec::<Error>::new()));

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert!(finished.out.len() <= BUDGET);
    let shown = finished.out.lines().count() - 1;
    assert_eq!(
        finished.err,
        format!(
            "error: {} more answers did not fit in the output budget of 1024 bytes\n  \
             help:  run `tool show {}`\n  \
             code:  tool::output::over_budget\n",
            20 - shown,
            names(shown + 1, 20).join(" ")
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
        json!({"left_out": ["star-03", "star-04"], "left_out_from": 2, "budget": 1024})
    );
}

#[test]
fn a_key_left_out_is_quoted_for_a_shell_in_text_and_repeats_as_often_as_it_was_given() {
    let keys = ["star-01", "Le Guin", "Le Guin"].map(str::to_owned);
    let finished = finish_small(&[], STAR_BATCH, fetched(&keys, stars(1)));

    assert!(
        finished
            .err
            .contains("help:  run `tool show 'Le Guin' 'Le Guin'`\n"),
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
        json!({"left_out": [], "left_out_from": shown, "unlisted": 20 - shown, "budget": 1024})
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
        format!(
            "99 more answers did not fit in the output budget of 1024 bytes. The recovery covers the keys listed in left_out, and a later run covers the other {}",
            99 - listed.len()
        )
    );
}

/// Returns a batch of three stars whose first row alone passes a budget of
/// 1024 bytes in text and with `--json`.
fn large_first() -> Batch<Star, Error> {
    let mut first = star(1);
    first.story = "a".repeat(1100);
    keyed(vec![first, star(2), star(3)], Vec::<Error>::new())
}

#[test]
fn a_batch_without_the_mark_refuses_a_first_answer_alone_past_the_budget_and_names_every_later_key()
{
    let json = finish_output(budgeted(1024, &["--json"]), STAR_BATCH, large_first());
    let text = finish_output(budgeted(1024, &[]), STAR_BATCH, large_first());

    assert_eq!(json.code, ExitCode::FAILURE);
    let document = parsed(&json.out);
    assert_eq!(document["data"], json!([]));
    let failures = document["failures"].as_array().unwrap();
    assert_eq!(failures.len(), 2, "{failures:?}");
    assert_eq!(failures[0]["key"], "star-01");
    assert_eq!(failures[0]["index"], 0);
    assert_eq!(failures[0]["fields"], json!({"budget": 1024}));
    assert_eq!(
        failures[0]["recovery"],
        json!({"kind": "hint", "text": "ask for fewer fields with --fields"})
    );
    assert_eq!(
        failures[1]["fields"],
        json!({"left_out": ["star-02", "star-03"], "left_out_from": 1, "budget": 1024})
    );
    assert_eq!(
        failures[1]["recovery"],
        json!({"kind": "run", "call": "tool show <NAME>...", "all": "left_out"})
    );
    assert_eq!(text.code, ExitCode::FAILURE);
    assert_eq!(text.out, "");
    assert_eq!(
        text.err,
        "\
error: the result is larger than the output budget of 1024 bytes
  key:   star-01 (index 0)
  help:  ask for fewer fields with --fields
  code:  tool::output::over_budget
error: 2 more answers did not fit in the output budget of 1024 bytes
  help:  run `tool show star-02 star-03`
  code:  tool::output::over_budget
"
    );
}

#[test]
fn a_table_batch_whose_first_answer_is_refused_prints_every_row_when_asked_for_fewer_fields() {
    let json = finish_output(
        budgeted(1024, &["--json", "--fields", "name"]),
        STAR_BATCH,
        large_first(),
    );
    let text = finish_output(
        budgeted(1024, &["--fields", "name"]),
        STAR_BATCH,
        large_first(),
    );

    assert_eq!(json.code, ExitCode::SUCCESS, "{}", json.out);
    let document = parsed(&json.out);
    assert_eq!(
        document["data"].as_array().unwrap().len(),
        3,
        "{}",
        json.out
    );
    assert_eq!(text.code, ExitCode::SUCCESS, "{}", text.err);
    assert_eq!(text.out.lines().count(), 4, "{}", text.out);
    assert_eq!(text.err, "");
}

#[test]
fn a_batch_that_always_prints_its_first_answer_prints_it_past_the_budget_and_names_every_later_key()
{
    let marked = STAR_BATCH.always_print_first();
    let json = finish_output(budgeted(1024, &["--json"]), marked, large_first());
    let text = finish_output(budgeted(1024, &[]), marked, large_first());

    assert_eq!(json.code, ExitCode::FAILURE);
    let document = parsed(&json.out);
    assert_eq!(document["data"].as_array().unwrap().len(), 1);
    assert_eq!(
        document["failures"],
        json!([{
            "code": "tool::output::over_budget",
            "message": "2 more answers did not fit in the output budget of 1024 bytes",
            "fields": {"left_out": ["star-02", "star-03"], "left_out_from": 1, "budget": 1024},
            "recovery": {"kind": "run", "call": "tool show <NAME>...", "all": "left_out"},
        }])
    );
    assert_eq!(text.code, ExitCode::FAILURE);
    assert!(text.out.starts_with("NAME"), "{}", text.out);
    assert_eq!(text.out.lines().count(), 2, "{}", text.out);
    assert!(
        text.err
            .starts_with("error: 2 more answers did not fit in the output budget of 1024 bytes\n"),
        "{}",
        text.err
    );
}

#[test]
fn the_failure_of_the_answers_left_out_gives_the_index_of_the_first_one_past_a_failure_that_printed()
 {
    let keys = names(1, 20);
    let mut batch = Batch::new(&["show"], "names", &keys);
    batch.push(Ok(star(1)));
    batch.push(Err(Dim(2)));
    batch.extend((3..=20).map(|number| Ok(star(number))));

    let finished = finish_small(&["--json"], DIM_BATCH, batch);

    let document = parsed(&finished.out);
    let shown = document["data"].as_array().unwrap().len();
    let fields = &document["failures"][1]["fields"];
    assert_eq!(fields["left_out_from"], shown + 1);
    assert_eq!(fields["left_out"][0], keys[shown + 1]);
}
