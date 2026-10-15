//! Tests for how the output budget cuts a document with the tool's own
//! envelope keys (ADR-0059).

use std::process::ExitCode;

use super::{BUDGET, STAR_BATCH, STAR_LISTING, finish_small, keyed, parsed, stars};
use crate::{Error, Listing};

#[test]
fn a_listing_with_its_own_keys_holds_cut_and_prints_no_cut_line() {
    let document = STAR_LISTING.with_keys("$type", "entries");
    let finished = finish_small(&["--json"], document, Listing::new(stars(20), 20));

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert!(finished.out.len() <= BUDGET, "{}", finished.out.len());
    let printed = parsed(&finished.out);
    assert_eq!(printed["$type"], "sky/star-listing/v1");
    assert_eq!(printed["cut"]["budget"], BUDGET);
    assert_eq!(printed["truncated"], true);
    assert_eq!(finished.err, "");
}

#[test]
fn a_batch_in_input_order_with_its_own_keys_ends_its_answers_with_the_failure_of_the_answers_left_out()
 {
    let document = STAR_BATCH.with_keys("$type", "answers").failures_in_order();
    let finished = finish_small(&["--json"], document, keyed(stars(20), Vec::<Error>::new()));

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert!(finished.out.len() <= BUDGET, "{}", finished.out.len());
    assert_eq!(finished.err, "");
    let printed = parsed(&finished.out);
    assert!(printed.get("failures").is_none(), "{}", finished.out);
    let answers = printed["answers"].as_array().unwrap();
    let (over, rows) = answers.split_last().unwrap();
    assert!(!rows.is_empty() && rows.len() < 20, "{}", rows.len());
    assert_eq!(over["code"], "crumbkit::output::over_budget");
    assert_eq!(over["fields"]["left_out_from"], rows.len());
}
