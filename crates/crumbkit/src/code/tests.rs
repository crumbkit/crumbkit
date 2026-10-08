//! Tests for failure codes and their spellings.

use std::fmt;

use super::{Code, Spelling};

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

#[test]
fn dots_spells_the_segments_joined_by_dots() {
    assert_eq!(Spelling::DOTS.spell(&NOT_FOUND), "shelf.books.not_found");
}

#[test]
fn slashes_spells_the_segments_joined_by_slashes() {
    assert_eq!(Spelling::SLASHES.spell(&NOT_FOUND), "shelf/books/not_found");
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
