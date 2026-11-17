//! Tests for failure codes and their spellings.

use std::panic;

use proptest::prelude::*;

use super::{Code, Segment, Spelling};
use crate::segment::is_code_segment;
use crate::{Error, ToolName};

const NOT_FOUND: Code<'static> = Code::from_spelling("shelf::books::not_found");

#[test]
fn spell_takes_the_code_a_failure_returns_and_writes_the_tool_s_name_in_crumbkit_s_codes() {
    let spelling = Spelling::COLONS.for_tool(ToolName::new("shelf"));
    let error = Error::unclassified("the disk is full");

    assert_eq!(
        spelling.spell(error.failure().code()),
        "shelf::report::unchecked"
    );
    assert_eq!(spelling.spell(NOT_FOUND), "shelf::books::not_found");
}

#[test]
fn a_code_keeps_its_component_area_and_reason() {
    assert_eq!(NOT_FOUND.component(), "shelf");
    assert_eq!(NOT_FOUND.area(), "books");
    assert_eq!(NOT_FOUND.reason(), "not_found");
}

#[test]
fn a_segment_may_hold_lowercase_letters_digits_and_underscores() {
    let code = Code::from_spelling("shelf_2::books_v2::not_found_3");

    assert_eq!(
        Spelling::COLONS.spell(code),
        "shelf_2::books_v2::not_found_3"
    );
}

#[test]
#[should_panic(expected = "component")]
fn an_empty_component_is_rejected() {
    let _ = Code::from_spelling("::books::not_found");
}

#[test]
#[should_panic(expected = "component")]
fn a_component_with_an_uppercase_letter_is_rejected() {
    let _ = Code::from_spelling("Shelf::books::not_found");
}

#[test]
#[should_panic(expected = "area")]
fn an_empty_area_is_rejected() {
    let _ = Code::from_spelling("shelf::::not_found");
}

#[test]
fn a_segment_may_hold_hyphens_between_words() {
    let code = Code::from_spelling("book-club::reading-list::not-found");

    assert_eq!(
        Spelling::COLONS.spell(code),
        "book-club::reading-list::not-found"
    );
}

#[test]
#[should_panic(expected = "component")]
fn a_component_with_a_leading_hyphen_is_rejected() {
    let _ = Code::from_spelling("-shelf::books::not_found");
}

#[test]
#[should_panic(expected = "area")]
fn an_area_with_a_trailing_hyphen_is_rejected() {
    let _ = Code::from_spelling("shelf::books-::not_found");
}

#[test]
#[should_panic(expected = "reason")]
fn a_reason_with_two_hyphens_in_a_row_is_rejected() {
    let _ = Code::from_spelling("shelf::books::not--found");
}

#[test]
#[should_panic(expected = "reason")]
fn an_empty_reason_is_rejected() {
    let _ = Code::from_spelling("shelf::books::");
}

#[test]
#[should_panic(expected = "reason")]
fn a_reason_with_a_space_is_rejected() {
    let _ = Code::from_spelling("shelf::books::not found");
}

#[test]
#[should_panic(expected = "reason")]
fn a_reason_with_a_letter_outside_ascii_is_rejected() {
    let _ = Code::from_spelling("shelf::books::introuvé");
}

#[test]
fn colons_spells_the_segments_joined_by_two_colons() {
    assert_eq!(Spelling::COLONS.spell(NOT_FOUND), "shelf::books::not_found");
}

/// Generates a segment that `Code::from_spelling` accepts.
const SEGMENT: &str = "[a-z0-9_]{1,4}(-[a-z0-9_]{1,4}){0,2}";

/// Generates strings of up to six characters drawn from the characters that a
/// code segment allows and characters that it does not: an upper-case letter,
/// a hyphen, which a segment allows only between two other characters, a
/// slash, a dot, a colon, a space and `é`. The test keeps only the strings
/// that are not segments.
const NEAR_SEGMENTS: &str = r"[a-z0-9_A\-/.: é]{0,6}";

/// The name of each segment of a code, in the order of its spelling.
const SEGMENT_NAMES: [&str; 3] = ["component", "area", "reason"];

/// Returns `text` as a `&'static str`, which a `const` spelling requires.
/// Each case leaks a few short strings, which the test process frees when it
/// exits.
fn leak(text: String) -> &'static str {
    text.leak()
}

proptest! {
    #[test]
    fn from_spelling_panics_on_a_segment_that_breaks_the_rule_and_names_that_segment(
        mut segments in prop::array::uniform3(SEGMENT),
        position in 0..3_usize,
        invalid in NEAR_SEGMENTS.prop_filter("the string is a segment", |text| {
            !is_code_segment(text) && !text.contains(':')
        }),
    ) {
        segments[position] = invalid;
        let spelling = leak(segments.join("::"));

        let outcome = panic::catch_unwind(|| Code::from_spelling(spelling));

        let payload =
            outcome.expect_err("Code::from_spelling accepted a segment that breaks the rule");
        // `panic!` with a literal message panics with a `&str`.
        let message = payload.downcast_ref::<&str>().copied().unwrap_or_default();
        let expected = format!("a code {} is ", SEGMENT_NAMES[position]);
        prop_assert!(message.starts_with(&expected), "{message}");
    }

    /// Splitting the text at `::` gives the segments back, so the text is
    /// the segments joined by `::`, and two different codes never spell the
    /// same: equal texts split into equal segments. The test builds every
    /// code from three generated segments, so it also shows that
    /// `Code::from_spelling` accepts any three segments that follow the rule.
    #[test]
    fn the_colon_spelling_is_printable_ascii_that_splits_back_into_the_segments(
        segments in prop::array::uniform3(SEGMENT),
    ) {
        let [component, area, reason] = segments.clone();
        let code = Code::from_spelling(leak(segments.join("::")));

        let text = Spelling::COLONS.spell(code);

        prop_assert!(
            text.chars().all(|character| matches!(character, '!'..='~')),
            "{text}"
        );
        prop_assert_eq!(
            text.split("::").collect::<Vec<_>>(),
            [component.as_str(), area.as_str(), reason.as_str()]
        );
        prop_assert_eq!(code.component(), component);
        prop_assert_eq!(code.area(), area);
        prop_assert_eq!(code.reason(), reason);
    }
}

#[test]
fn from_spelling_splits_a_spelling_into_its_component_area_and_reason() {
    let code = Code::from_spelling("shelf::books::not_found");

    assert_eq!(
        (code.component(), code.area(), code.reason()),
        ("shelf", "books", "not_found")
    );
}

#[test]
#[should_panic(expected = "three segments")]
fn a_spelling_with_two_parts_is_rejected() {
    let _ = Code::from_spelling("shelf::not_found");
}

#[test]
#[should_panic(expected = "three segments")]
fn a_spelling_with_four_parts_is_rejected() {
    let _ = Code::from_spelling("shelf::books::not::found");
}

#[test]
#[should_panic(expected = "three segments")]
fn a_spelling_with_a_single_colon_in_place_of_a_separator_is_rejected() {
    let _ = Code::from_spelling("shelf:books::not_found");
}

#[test]
fn as_str_returns_the_declared_spelling_with_a_static_lifetime() {
    let text: &'static str = NOT_FOUND.as_str();

    assert_eq!(text, "shelf::books::not_found");
}

#[test]
fn two_codes_with_the_same_spelling_are_equal_and_hash_the_same() {
    use std::hash::BuildHasher;

    let borrowed = String::from("shelf::books::not_found");
    let runtime = Code::runtime(&borrowed).unwrap();
    let hasher = std::collections::hash_map::RandomState::new();

    assert_eq!(runtime, NOT_FOUND);
    assert_eq!(hasher.hash_one(runtime), hasher.hash_one(NOT_FOUND));
    assert_ne!(NOT_FOUND, Code::from_spelling("shelf::books::gone"));
}

#[test]
fn a_crumbkit_code_prints_with_the_tool_s_component_and_as_str_keeps_crumbkit() {
    const USAGE: Code<'static> = Code::from_spelling("crumbkit::cli::usage");
    let spelling = Spelling::COLONS.for_tool(ToolName::new("shelf"));

    assert_eq!(spelling.spell(USAGE), "shelf::cli::usage");
    assert_eq!(USAGE.as_str(), "crumbkit::cli::usage");
}

#[test]
fn the_three_segments_display_as_component_area_and_reason() {
    assert_eq!(Segment::Component.to_string(), "component");
    assert_eq!(Segment::Area.to_string(), "area");
    assert_eq!(Segment::Reason.to_string(), "reason");
}
