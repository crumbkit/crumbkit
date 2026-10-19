//! Tests for failure codes and their spellings.

use std::fmt;
use std::panic;

use proptest::prelude::*;

use super::{Code, Form, Spelling};
use crate::segment::is_segment;

const NOT_FOUND: Code = Code::new("shelf", "books", "not_found");

#[test]
fn a_code_keeps_its_component_area_and_reason() {
    assert_eq!(NOT_FOUND.component(), "shelf");
    assert_eq!(NOT_FOUND.area(), "books");
    assert_eq!(NOT_FOUND.reason(), "not_found");
}

#[test]
fn a_segment_may_hold_lowercase_letters_digits_and_underscores() {
    let code = Code::new("shelf_2", "books_v2", "not_found_3");

    assert_eq!(
        Spelling::COLONS.spell(&code),
        "shelf_2::books_v2::not_found_3"
    );
}

#[test]
#[should_panic(expected = "component")]
fn an_empty_component_is_rejected() {
    let _ = Code::new("", "books", "not_found");
}

#[test]
#[should_panic(expected = "component")]
fn a_component_with_an_uppercase_letter_is_rejected() {
    let _ = Code::new("Shelf", "books", "not_found");
}

#[test]
#[should_panic(expected = "area")]
fn an_empty_area_is_rejected() {
    let _ = Code::new("shelf", "", "not_found");
}

#[test]
#[should_panic(expected = "area")]
fn an_area_with_a_dash_is_rejected() {
    let _ = Code::new("shelf", "book-list", "not_found");
}

#[test]
#[should_panic(expected = "reason")]
fn an_empty_reason_is_rejected() {
    let _ = Code::new("shelf", "books", "");
}

#[test]
#[should_panic(expected = "reason")]
fn a_reason_with_a_space_is_rejected() {
    let _ = Code::new("shelf", "books", "not found");
}

#[test]
#[should_panic(expected = "reason")]
fn a_reason_with_a_letter_outside_ascii_is_rejected() {
    let _ = Code::new("shelf", "books", "introuvé");
}

#[test]
fn colons_spells_the_segments_joined_by_two_colons() {
    assert_eq!(
        Spelling::COLONS.spell(&NOT_FOUND),
        "shelf::books::not_found"
    );
}

/// Spells a code as its area and reason in upper case, joined by a dash.
fn upper_area_and_reason(code: &Code, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
        f,
        "{}-{}",
        code.area().to_uppercase(),
        code.reason().to_uppercase()
    )
}

#[test]
fn a_custom_spelling_spells_a_code_with_the_function_it_was_given() {
    const UPPER: Spelling = Spelling::custom(upper_area_and_reason);

    assert_eq!(UPPER.spell(&NOT_FOUND), "BOOKS-NOT_FOUND");
}

/// Generates a segment that `Code::new` accepts.
const SEGMENT: &str = "[a-z0-9_]+";

/// Generates strings of up to six characters drawn from the characters that a
/// code segment allows and characters that it does not: an upper-case letter,
/// a dash, a slash, a dot, a colon, a space and `é`. The test keeps only the
/// strings that are not segments.
const NEAR_SEGMENTS: &str = r"[a-z0-9_A\-/.: é]{0,6}";

/// The name of each segment of a code, in the order `Code::new` takes them.
const SEGMENT_NAMES: [&str; 3] = ["component", "area", "reason"];

/// Returns `text` as a `&'static str`, which `Code::new` requires. Each case
/// leaks a few short strings, which the test process frees when it exits.
fn leak(text: String) -> &'static str {
    text.leak()
}

proptest! {
    #[test]
    fn new_panics_on_a_segment_that_breaks_the_rule_and_names_that_segment(
        mut segments in prop::array::uniform3(SEGMENT),
        position in 0..3_usize,
        invalid in NEAR_SEGMENTS.prop_filter("the string is a segment", |text| {
            !is_segment(text, b'_')
        }),
    ) {
        segments[position] = invalid;
        let [component, area, reason] = segments.map(leak);

        let outcome = panic::catch_unwind(|| Code::new(component, area, reason));

        let payload =
            outcome.expect_err("Code::new accepted a segment that breaks the rule");
        // `assert!` with a literal message panics with a `&str`.
        let message = payload.downcast_ref::<&str>().copied().unwrap_or_default();
        let expected = format!("a code {} is ", SEGMENT_NAMES[position]);
        prop_assert!(message.starts_with(&expected), "{message}");
    }

    /// Splitting the text at `::` gives the segments back, so the text is
    /// the segments joined by `::`, and two different codes never spell the
    /// same: equal texts split into equal segments. The test builds every
    /// code from three generated segments, so it also shows that `Code::new`
    /// accepts any three segments that follow the rule.
    #[test]
    fn the_colon_spelling_is_printable_ascii_that_splits_back_into_the_segments(
        segments in prop::array::uniform3(SEGMENT),
    ) {
        let [component, area, reason] = segments.map(leak);
        let code = Code::new(component, area, reason);

        let text = Spelling::COLONS.spell(&code);

        prop_assert!(
            text.chars().all(|character| matches!(character, '!'..='~')),
            "{text}"
        );
        prop_assert_eq!(
            text.split("::").collect::<Vec<_>>(),
            [component, area, reason]
        );
    }
}

#[test]
fn the_colon_spelling_splits_a_spelled_code_into_its_segments() {
    let text = Spelling::COLONS.spell(&NOT_FOUND);

    assert_eq!(
        Spelling::COLONS.split(&text),
        Some(("shelf", "books", "not_found"))
    );
}

#[test]
fn split_returns_none_for_text_that_is_not_three_parts() {
    assert_eq!(Spelling::COLONS.split("shelf::books"), None);
    assert_eq!(Spelling::COLONS.split("shelf::books::not::found"), None);
    assert_eq!(Spelling::COLONS.split("shelf.books.not_found"), None);
}

#[test]
fn a_spelling_for_a_tool_splits_as_the_spelling_it_names() {
    let spelling = Spelling::COLONS.for_tool(crate::ToolName::new("shelf"));

    assert_eq!(
        spelling.split("lending::loans::overdue"),
        Some(("lending", "loans", "overdue"))
    );
}

#[test]
fn a_custom_spelling_splits_nothing() {
    const UPPER: Spelling = Spelling::custom(upper_area_and_reason);

    assert_eq!(UPPER.split("BOOKS-NOT_FOUND"), None);
}

/// A joined spelling holds its separator once, so the text it writes and
/// the text it splits agree for any separator.
#[test]
fn a_joined_spelling_splits_at_the_separator_it_writes() {
    let dots = Spelling {
        form: Form::Joined("."),
        tool: crate::ToolName::CRUMBKIT,
    };

    let text = dots.spell(&NOT_FOUND);

    assert_eq!(text, "shelf.books.not_found");
    assert_eq!(dots.split(&text), Some(("shelf", "books", "not_found")));
}
