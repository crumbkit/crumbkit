//! Tests for a leaf whose command prints no result on standard output.

use crate::help::{Leaf, ViewFlag};

/// A server that runs until it is interrupted and prints no result.
const SERVE: Leaf = Leaf::new(&["serve"]).prints_no_result();

#[test]
fn a_leaf_that_prints_no_result_offers_no_view_flag() {
    assert_eq!(
        ViewFlag::ALL.map(|flag| SERVE.offers(flag)),
        [false, false, false]
    );
}

#[test]
fn a_leaf_without_a_document_that_does_not_declare_no_result_still_offers_every_view_flag() {
    let plain = Leaf::new(&["serve"]);

    assert_eq!(
        ViewFlag::ALL.map(|flag| plain.offers(flag)),
        [true, true, true]
    );
}

#[test]
fn a_leaf_that_prints_no_result_reports_it_and_has_no_document_or_schema() {
    assert!(SERVE.get_prints_no_result());
    assert!(!Leaf::new(&["serve"]).get_prints_no_result());
    assert!(SERVE.get_document().is_none());
    assert!(SERVE.document_schema().is_none());
}

#[test]
fn a_leaf_that_prints_no_result_has_the_no_result_output_line() {
    assert_eq!(
        SERVE.output_line(),
        Some("Output: no result on standard output, and --json prints a failure as JSON.")
    );
    assert_eq!(Leaf::new(&["serve"]).output_line(), None);
}
