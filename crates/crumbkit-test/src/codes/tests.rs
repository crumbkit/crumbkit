//! Tests for the codes check.

use std::fmt;

use crumbkit::help::Guide;
use crumbkit::{Code, Spelling};

use super::check_codes;
use crate::testing::panic_message;

const NOT_FOUND: Code = Code::new("shelf", "books", "not_found");
const TIMED_OUT: Code = Code::new("shelf", "network", "timed_out");
const AUTHOR_NOT_FOUND: Code = Code::new("shelf", "authors", "not_found");

/// Spells only the component and the reason, so codes in different areas
/// can spell the same.
fn without_area(code: &Code, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}::{}", code.component(), code.reason())
}

/// Spells every code as nothing.
fn empty(_: &Code, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("")
}

/// Spells a code with spaces between its segments.
fn spaced(code: &Code, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{} {} {}", code.component(), code.area(), code.reason())
}

/// Spells a code with the delete control character between its segments.
fn with_delete(code: &Code, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
        f,
        "{}\u{7f}{}\u{7f}{}",
        code.component(),
        code.area(),
        code.reason()
    )
}

/// Spells a code with arrows between its segments.
fn arrows(code: &Code, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}→{}→{}", code.component(), code.area(), code.reason())
}

#[test]
fn distinct_codes_in_the_colon_spelling_of_a_guide_without_a_spelling_pass() {
    check_codes(
        &Guide::new(),
        &[&[&NOT_FOUND, &TIMED_OUT], &[&AUTHOR_NOT_FOUND]],
    );
}

#[test]
fn the_same_code_listed_by_two_error_types_passes() {
    check_codes(&Guide::new(), &[&[&NOT_FOUND], &[&NOT_FOUND, &TIMED_OUT]]);
}

#[test]
fn two_codes_that_spell_the_same_fail_and_both_are_named() {
    let message = panic_message(|| {
        check_codes(
            &Guide::new().spelling(Spelling::custom(without_area)),
            &[&[&NOT_FOUND], &[&AUTHOR_NOT_FOUND]],
        );
    })
    .unwrap();

    assert!(
        message.contains(
            r#"Code::new("shelf", "books", "not_found") and Code::new("shelf", "authors", "not_found") both spell "shelf::not_found""#
        ),
        "{message}"
    );
}

#[test]
fn a_code_that_spells_as_an_empty_string_fails() {
    let message = panic_message(|| {
        check_codes(
            &Guide::new().spelling(Spelling::custom(empty)),
            &[&[&NOT_FOUND]],
        );
    })
    .unwrap();

    assert!(
        message.contains(r#"Code::new("shelf", "books", "not_found") spells as an empty string"#),
        "{message}"
    );
}

#[test]
fn a_code_that_spells_with_a_space_fails() {
    let message = panic_message(|| {
        check_codes(
            &Guide::new().spelling(Spelling::custom(spaced)),
            &[&[&NOT_FOUND]],
        );
    })
    .unwrap();

    assert!(
        message.contains(
            r#"Code::new("shelf", "books", "not_found") spells as "shelf books not_found", which holds a character other than printable ASCII"#
        ),
        "{message}"
    );
}

#[test]
fn a_code_that_spells_with_a_control_character_fails() {
    let message = panic_message(|| {
        check_codes(
            &Guide::new().spelling(Spelling::custom(with_delete)),
            &[&[&NOT_FOUND]],
        );
    })
    .unwrap();

    assert!(
        message.contains(
            r#"Code::new("shelf", "books", "not_found") spells as "shelf\u{7f}books\u{7f}not_found", which holds a character other than printable ASCII"#
        ),
        "{message}"
    );
}

#[test]
fn a_code_that_spells_with_a_character_outside_ascii_fails() {
    let message = panic_message(|| {
        check_codes(
            &Guide::new().spelling(Spelling::custom(arrows)),
            &[&[&NOT_FOUND]],
        );
    })
    .unwrap();

    assert!(
        message.contains(
            r#"Code::new("shelf", "books", "not_found") spells as "shelf→books→not_found", which holds a character other than printable ASCII"#
        ),
        "{message}"
    );
}

#[test]
fn one_failed_check_names_every_problem() {
    let message = panic_message(|| {
        check_codes(
            &Guide::new().spelling(Spelling::custom(empty)),
            &[&[&NOT_FOUND, &TIMED_OUT]],
        );
    })
    .unwrap();

    assert!(
        message.contains(r#"Code::new("shelf", "books", "not_found") spells as an empty string"#),
        "{message}"
    );
    assert!(
        message.contains(r#"Code::new("shelf", "network", "timed_out") spells as an empty string"#),
        "{message}"
    );
    assert!(message.contains(r#"both spell """#), "{message}");
}
