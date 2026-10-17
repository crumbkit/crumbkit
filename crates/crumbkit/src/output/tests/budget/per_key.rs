//! Tests for how `finish_to` prints a batch whose document answers each key
//! that the output budget leaves out with a failure of its own (ADR-0068).

use std::process::ExitCode;

use proptest::prelude::*;
use serde_json::{Value, json};

use super::{
    BUDGET, DIM_BATCH, Dim, RENAMED, STAR_BATCH, Star, budgeted, finish_small, guided, parsed, star,
};
use crate::help::Guide;
use crate::output::tests::finish_output;
use crate::{Batch, Error, JsonLayout};

/// Returns a star whose story is `length` bytes long.
fn long_star(number: usize, length: usize) -> Star {
    Star {
        story: "a".repeat(length),
        ..star(number)
    }
}

/// Returns the index and code of each failure of a batch's JSON document.
fn indexed_codes(document: &Value) -> Vec<(u64, String)> {
    document["failures"]
        .as_array()
        .unwrap()
        .iter()
        .map(|failure| {
            let index = failure["index"].as_u64().unwrap();
            (index, failure["code"].as_str().unwrap().to_owned())
        })
        .collect()
}

#[test]
fn a_batch_that_answers_each_left_out_key_prints_one_over_budget_failure_at_each_key_s_position() {
    let keys: Vec<String> = (1..=6).map(|number| star(number).name).collect();
    let mut batch = Batch::new(&["show"], "names", &keys);
    batch.push(Ok(long_star(1, 300)));
    batch.push(Ok(long_star(2, 300)));
    batch.push(Err(Dim(3)));
    batch.push(Ok(long_star(4, 300)));
    batch.push(Ok(long_star(5, 300)));

    let finished = finish_output(
        &mut budgeted(1600, &["--json"]),
        DIM_BATCH.left_out_per_key(),
        batch,
    );

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert!(finished.out.len() <= 1600, "{}", finished.out.len());
    let document = parsed(&finished.out);
    let shown = document["data"].as_array().unwrap().len();
    assert_eq!(shown, 3, "{}", finished.out);
    let over = "crumbkit::output::over_budget".to_owned();
    assert_eq!(
        indexed_codes(&document),
        [
            (2, "sky::stars::too_much".to_owned()),
            (4, over.clone()),
            (5, over)
        ]
    );
    assert_eq!(
        document["failures"][1],
        json!({
            "key": "star-05",
            "index": 4,
            "code": "crumbkit::output::over_budget",
            "message": "the answer did not fit in the output budget of 1600 bytes",
            "fields": {"budget": 1600},
            "recovery": {"kind": "run", "call": "tool show star-05"}
        })
    );
}

#[test]
fn the_per_key_fit_prints_the_most_rows_when_a_row_is_smaller_than_its_failure() {
    let keys: Vec<String> = (1..=8).map(|number| star(number).name).collect();
    let fetched = || {
        let mut batch: Batch<Star, Error> = Batch::new(&["show"], "names", &keys);
        batch.extend((1..=4).map(|number| Ok(star(number))));
        batch
    };
    let document = STAR_BATCH.left_out_per_key();
    // A budget of four digits prints as many bytes in each failure as the
    // budget that the rows and failures fill exactly.
    let roomy = finish_output(&mut budgeted(9999, &["--json"]), document, fetched());
    let budget = roomy.out.len();
    assert!((BUDGET..9999).contains(&budget), "{budget}");

    let finished = finish_output(&mut budgeted(budget, &["--json"]), document, fetched());

    assert_eq!(finished.out.len(), budget, "{}", finished.out);
    let printed = parsed(&finished.out);
    assert_eq!(printed["data"].as_array().unwrap().len(), 4);
    assert_eq!(printed["failures"].as_array().unwrap().len(), 4);
}

#[test]
fn each_left_out_key_s_failure_runs_the_command_with_that_key_alone() {
    let keys: Vec<String> = (1..=4).map(|number| star(number).name).collect();
    let mut batch: Batch<Star, Error> = Batch::new(&["show"], "names", &keys);
    batch.extend((1..=2).map(|number| Ok(star(number))));

    let finished = finish_small(&[], STAR_BATCH.left_out_per_key(), batch);

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert_eq!(
        finished.err,
        "\
error: the answer did not fit in the output budget of 1024 bytes
  key:   star-03 (index 2)
  help:  run `tool show star-03`
  code:  crumbkit::output::over_budget
error: the answer did not fit in the output budget of 1024 bytes
  key:   star-04 (index 3)
  help:  run `tool show star-04`
  code:  crumbkit::output::over_budget
"
    );
}

#[test]
fn the_first_failure_in_key_order_decides_the_exit_of_a_batch_that_answers_each_left_out_key() {
    let keys = ["star-01", "star-02", "missing"];
    let batch = || {
        let mut batch: Batch<Star, Error> = Batch::new(&["show"], "names", &keys);
        batch.push(Ok(long_star(1, 600)));
        batch.push(Ok(long_star(2, 600)));
        batch.push(Err(Error::unclassified("no star is named missing")));
        batch
    };

    let per_key = finish_output(
        &mut guided(&RENAMED, &[]),
        STAR_BATCH.left_out_per_key(),
        batch(),
    );
    let one_failure = finish_output(&mut guided(&RENAMED, &[]), STAR_BATCH, batch());

    assert_eq!(per_key.code, ExitCode::from(9));
    assert_eq!(one_failure.code, ExitCode::FAILURE);
    assert!(
        per_key.err.find("key:   star-02") < per_key.err.find("key:   missing"),
        "{}",
        per_key.err
    );
}

#[test]
fn a_batch_in_input_order_answers_each_left_out_key_among_its_rows() {
    let document = STAR_BATCH
        .without_envelope()
        .failures_in_order()
        .left_out_per_key();
    let keys: Vec<String> = (1..=3).map(|number| star(number).name).collect();
    let mut batch: Batch<Star, Error> = Batch::new(&["show"], "names", &keys);
    batch.push(Ok(star(1)));

    let finished = finish_small(&["--json"], document, batch);

    assert_eq!(finished.err, "");
    let array = parsed(&finished.out);
    let indexes: Vec<&Value> = array
        .as_array()
        .unwrap()
        .iter()
        .map(|element| &element["index"])
        .collect();
    assert_eq!(indexes, [&Value::Null, &json!(1), &json!(2)]);
}

#[test]
fn a_batch_that_answers_each_left_out_key_prints_each_failure_even_when_they_pass_the_budget() {
    let keys: Vec<String> = (1..=20).map(|number| star(number).name).collect();
    let mut batch: Batch<Star, Error> = Batch::new(&["show"], "names", &keys);
    batch.push(Ok(star(1)));

    let finished = finish_small(&["--json"], STAR_BATCH.left_out_per_key(), batch);

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert!(finished.out.len() > BUDGET, "{}", finished.out.len());
    let document = parsed(&finished.out);
    assert_eq!(document["data"].as_array().unwrap().len(), 1);
    let over = "crumbkit::output::over_budget".to_owned();
    let expected: Vec<(u64, String)> = (1..20).map(|index| (index, over.clone())).collect();
    assert_eq!(indexed_codes(&document), expected);
}

#[test]
fn a_batch_of_three_large_rows_answers_its_second_and_third_keys_with_their_own_failures_in_a_pretty_document()
 {
    const GUIDE: Guide = Guide::new()
        .output_budget(BUDGET, "the tests print little")
        .json_layout(JsonLayout::Pretty, "the readers expect it");
    let keys: Vec<String> = (1..=3).map(|number| star(number).name).collect();
    let mut batch: Batch<Star, Error> = Batch::new(&["show"], "names", &keys);
    batch.extend((1..=3).map(|number| Ok(long_star(number, 500))));

    let finished = finish_output(
        &mut guided(&GUIDE, &["--json"]),
        STAR_BATCH.left_out_per_key(),
        batch,
    );

    assert_eq!(finished.code, ExitCode::FAILURE);
    let document = parsed(&finished.out);
    assert_eq!(document["data"].as_array().unwrap().len(), 1);
    let over = "crumbkit::output::over_budget".to_owned();
    assert_eq!(
        indexed_codes(&document),
        [(1, over.clone()), (2, over)],
        "{}",
        finished.out
    );
}

/// Returns the number of rows and the number of failures in the JSON of a
/// batch, whose rows and failures print in one array under `answers` when
/// `in_order` is true.
fn answers(document: &Value, in_order: bool) -> (usize, usize) {
    if in_order {
        let answers = document["answers"].as_array().unwrap();
        let failures = answers
            .iter()
            .filter(|answer| answer["code"].is_string())
            .count();
        (answers.len() - failures, failures)
    } else {
        let rows = document["data"].as_array().unwrap().len();
        (rows, document["failures"].as_array().unwrap().len())
    }
}

proptest! {
    #[test]
    fn a_batch_that_answers_each_left_out_key_answers_every_key_once_and_passes_the_budget_only_with_its_first_row(
        lengths in proptest::collection::vec(0_usize..600, 1..12),
        unfetched in 0_usize..4,
        budget in 1024_usize..4096,
        pretty in any::<bool>(),
        in_order in any::<bool>(),
        dim in 0_usize..3,
    ) {
        let mut keys: Vec<String> = (0..lengths.len() + dim + unfetched)
            .map(|number| format!("star-{number}"))
            .collect();
        keys.push("star-last".to_owned());
        let mut batch = Batch::new(&["show"], "names", &keys);
        batch.extend(lengths.iter().map(|&length| Ok(long_star(1, length))));
        batch.extend((0..dim).map(|number| Err(Dim(number))));
        let mut guide = Guide::new().output_budget(budget, "the tests print little");
        if pretty {
            guide = guide.json_layout(JsonLayout::Pretty, "the readers expect it");
        }
        let document = if in_order {
            DIM_BATCH.with_keys("$type", "answers").failures_in_order()
        } else {
            DIM_BATCH
        };

        let finished = finish_output(
            &mut guided(&guide, &["--json"]),
            document.left_out_per_key(),
            batch,
        );

        let (rows, failures) = answers(&parsed(&finished.out), in_order);
        prop_assert_eq!(rows + failures, keys.len(), "{}", finished.out);
        if finished.out.len() > budget {
            prop_assert_eq!(rows, 1, "{}", finished.out);
        }
    }
}

/// A form whose JSON says whether each failure is among several keys.
struct SeveralForm;

impl crate::form::FailureForm for SeveralForm {
    fn document(&self) -> crate::Document<()> {
        crate::Document::new("sky", "error", 1, "A failed command.")
    }

    fn schema(&self) -> schemars::Schema {
        schemars::json_schema!({ "type": "object" })
    }

    fn json(&self, failure: &crate::form::PreparedFailure<'_>) -> Value {
        json!({ "code": failure.code(), "several": failure.among_several() })
    }
}

#[test]
fn a_form_learns_that_each_left_out_key_s_failure_is_among_several_keys() {
    const GUIDE: Guide = Guide::new()
        .output_budget(BUDGET, "the tests print little")
        .failure_form(&SeveralForm);
    let keys: Vec<String> = (1..=4).map(|number| star(number).name).collect();
    let mut batch: Batch<Star, Error> = Batch::new(&["show"], "names", &keys);
    batch.extend((1..=2).map(|number| Ok(star(number))));

    let finished = finish_output(
        &mut guided(&GUIDE, &["--json"]),
        STAR_BATCH
            .with_keys("$type", "answers")
            .failures_in_order()
            .left_out_per_key(),
        batch,
    );

    let document = parsed(&finished.out);
    let answers = document["answers"].as_array().unwrap();
    let failures: Vec<&Value> = answers
        .iter()
        .filter(|answer| answer["code"].is_string())
        .collect();
    assert!(!failures.is_empty(), "{}", finished.out);
    for failure in failures {
        assert_eq!(failure["several"], json!(true), "{}", finished.out);
    }
}
