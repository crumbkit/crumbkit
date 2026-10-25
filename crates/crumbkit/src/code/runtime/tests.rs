//! Tests for failure codes known only at run time.

use std::ptr;

use proptest::prelude::*;

use super::{RuntimeCodeError, Table};
use crate::Code;
use crate::segment::is_code_segment;

#[test]
fn runtime_returns_a_code_with_the_segments_it_was_given() {
    let code = Code::runtime("plugin", "sync", "timed_out").unwrap();

    assert_eq!(code.component(), "plugin");
    assert_eq!(code.area(), "sync");
    assert_eq!(code.reason(), "timed_out");
}

#[test]
fn a_runtime_code_whose_component_names_a_hyphenated_plugin_spells_as_itself() {
    let code = Code::runtime("book-club", "gate", "unmet").unwrap();

    assert_eq!(
        crate::Spelling::COLONS.spell(code),
        "book-club::gate::unmet"
    );
}

#[test]
fn runtime_refuses_a_segment_that_starts_with_a_hyphen() {
    let error = Code::runtime("-book-club", "gate", "unmet").unwrap_err();

    assert_eq!(
        error,
        RuntimeCodeError::Segment {
            segment: "component",
            text: "-book-club".to_owned(),
        }
    );
}

#[test]
fn runtime_returns_the_same_code_each_time_it_is_given_the_same_segments() {
    let first = Code::runtime("plugin", "sync", "conflict").unwrap();
    let second = Code::runtime("plugin", "sync", "conflict").unwrap();

    assert!(ptr::eq(first, second));
}

#[test]
fn a_runtime_code_equals_the_constant_with_the_same_segments() {
    const CONFLICT: Code = Code::new("plugin", "sync", "conflict");

    assert_eq!(Code::runtime("plugin", "sync", "conflict"), Ok(&CONFLICT));
}

#[test]
fn a_full_table_refuses_a_new_code_and_still_returns_the_codes_it_holds() {
    let mut table = Table::new();
    let first = table.code("plugin", "sync", "one", 2).unwrap();
    table.code("plugin", "sync", "two", 2).unwrap();

    assert_eq!(
        table.code("plugin", "sync", "three", 2),
        Err(RuntimeCodeError::Full)
    );
    assert!(ptr::eq(
        table.code("plugin", "sync", "one", 2).unwrap(),
        first
    ));
}

#[test]
fn the_error_of_a_bad_segment_names_the_segment_and_its_text() {
    let error = Code::runtime("plugin", "Sync", "timed_out").unwrap_err();

    assert_eq!(
        error.to_string(),
        "the code area \"Sync\" is not words of lowercase ASCII letters, digits and underscores joined by single hyphens"
    );
}

#[test]
fn the_error_of_a_full_table_names_the_limit() {
    assert_eq!(
        RuntimeCodeError::Full.to_string(),
        "the process already holds 256 codes known only at run time, the most it can hold"
    );
}

/// The name of each segment of a code, in the order `Code::runtime` takes
/// them.
const SEGMENT_NAMES: [&str; 3] = ["component", "area", "reason"];

proptest! {
    /// `Code::runtime` applies the rule of `Code::new`: it refuses exactly
    /// the segments that `Code::new` would refuse to compile, and names the
    /// first one.
    #[test]
    fn runtime_refuses_exactly_the_segments_that_break_the_rule(
        segments in prop::array::uniform3(r"[a-z0-9_A\-/.: é]{0,6}"),
    ) {
        let [component, area, reason] = &segments;
        let first_bad = segments.iter().position(|text| !is_code_segment(text));

        let outcome = Table::new().code(component, area, reason, 1);

        match first_bad {
            None => prop_assert!(outcome.is_ok()),
            Some(position) => prop_assert_eq!(
                outcome,
                Err(RuntimeCodeError::Segment {
                    segment: SEGMENT_NAMES[position],
                    text: segments[position].clone(),
                })
            ),
        }
    }
}
