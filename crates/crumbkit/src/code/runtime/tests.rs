//! Tests for failure codes known only at run time.

use proptest::prelude::*;

use super::{RuntimeCodeError, Segment};
use crate::Code;
use crate::segment::is_code_segment;

#[test]
fn runtime_returns_a_code_with_the_segments_it_was_given() {
    let code = Code::runtime("plugin::sync::timed_out").unwrap();

    assert_eq!(code.component(), "plugin");
    assert_eq!(code.area(), "sync");
    assert_eq!(code.reason(), "timed_out");
}

#[test]
fn a_runtime_code_whose_component_names_a_hyphenated_plugin_spells_as_itself() {
    let code = Code::runtime("book-club::gate::unmet").unwrap();

    assert_eq!(
        crate::Spelling::COLONS.spell(code),
        "book-club::gate::unmet"
    );
}

#[test]
fn runtime_refuses_a_segment_that_starts_with_a_hyphen() {
    let error = Code::runtime("-book-club::gate::unmet").unwrap_err();

    assert_eq!(
        error,
        RuntimeCodeError::Segment {
            segment: Segment::Component,
            text: "-book-club".to_owned(),
        }
    );
}

#[test]
fn the_three_segments_display_as_component_area_and_reason() {
    assert_eq!(Segment::Component.to_string(), "component");
    assert_eq!(Segment::Area.to_string(), "area");
    assert_eq!(Segment::Reason.to_string(), "reason");
}

#[test]
fn three_hundred_distinct_runtime_codes_in_one_process_all_succeed() {
    for index in 0..300 {
        let reason = format!("reason_{index}");

        let text = format!("plugin::sync::{reason}");
        let code = Code::runtime(&text).unwrap();

        assert_eq!(
            crate::Spelling::COLONS.spell(code),
            format!("plugin::sync::{reason}")
        );
    }
}

#[test]
fn a_runtime_code_borrows_the_text_it_was_made_from() {
    let answer = String::from("lending::loans::overdue");

    let code = Code::runtime(&answer).unwrap();

    assert!(std::ptr::eq(code.as_str(), answer.as_str()));
    assert_eq!(code.component(), "lending");
    assert_eq!(code.area(), "loans");
    assert_eq!(code.reason(), "overdue");
}

#[test]
fn a_runtime_code_equals_the_constant_with_the_same_segments() {
    const CONFLICT: Code<'static> = Code::from_spelling("plugin::sync::conflict");

    assert_eq!(Code::runtime("plugin::sync::conflict"), Ok(CONFLICT));
}

#[test]
fn an_owned_code_lends_the_same_code_it_was_made_from() {
    let code = Code::runtime("plugin::sync::conflict").unwrap();

    assert_eq!(code.to_buf().as_code(), code);
}

#[test]
fn an_owned_code_outlives_the_text_it_was_made_from() {
    let owned = {
        let answer = String::from("lending::loans::overdue");
        Code::runtime(&answer).unwrap().to_buf()
    };

    assert_eq!(
        crate::Spelling::COLONS.spell(owned.as_code()),
        "lending::loans::overdue"
    );
}

#[test]
fn two_owned_codes_with_the_same_segments_are_equal() {
    let text = String::from("plugin::sync::conflict");
    let first = Code::runtime("plugin::sync::conflict").unwrap().to_buf();
    let second = Code::runtime(&text).unwrap().to_buf();

    assert_eq!(first, second);
}

#[test]
fn the_error_of_a_bad_segment_names_the_segment_and_its_text() {
    let error = Code::runtime("plugin::Sync::timed_out").unwrap_err();

    assert_eq!(
        error.to_string(),
        "the code area \"Sync\" is not words of lowercase ASCII letters, digits and underscores joined by single hyphens"
    );
}

/// Each segment of a code, in the order of its spelling.
const SEGMENTS: [Segment; 3] = [Segment::Component, Segment::Area, Segment::Reason];

#[test]
fn runtime_refuses_text_that_does_not_split_into_three_parts() {
    for text in [
        "",
        "shelf",
        "shelf::books",
        "shelf::books::not::found",
        "shelf:books:x",
    ] {
        assert_eq!(
            Code::runtime(text),
            Err(RuntimeCodeError::Parts {
                text: text.to_owned(),
            }),
            "{text:?}"
        );
    }
}

#[test]
fn the_error_of_text_with_the_wrong_parts_names_the_text() {
    let error = Code::runtime("shelf::books").unwrap_err();

    assert_eq!(
        error.to_string(),
        "the code \"shelf::books\" is not three segments joined by `::`, such as `shelf::books::not_found`"
    );
}

proptest! {
    /// `Code::runtime` applies the rule of `Code::from_spelling`: it refuses
    /// exactly the segments that `Code::from_spelling` would refuse to
    /// compile, and names the first one.
    #[test]
    fn runtime_refuses_exactly_the_segments_that_break_the_rule(
        segments in prop::array::uniform3(r"[a-z0-9_A\-/. é]{0,6}"),
    ) {
        let first_bad = segments.iter().position(|text| !is_code_segment(text));
        let spelling = segments.join("::");

        let outcome = Code::runtime(&spelling);

        match first_bad {
            None => prop_assert!(outcome.is_ok()),
            Some(position) => prop_assert_eq!(
                outcome,
                Err(RuntimeCodeError::Segment {
                    segment: SEGMENTS[position],
                    text: segments[position].clone(),
                })
            ),
        }
    }
}
