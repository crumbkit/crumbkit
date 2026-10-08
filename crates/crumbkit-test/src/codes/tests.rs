//! Tests for the codes check.

use std::fmt;
use std::panic;

use crumbkit::{Code, Spelling};

use super::check_codes;

const NOT_FOUND: Code = Code::new("shelf", "books", "not_found");
const TIMED_OUT: Code = Code::new("shelf", "network", "timed_out");
const AUTHOR_NOT_FOUND: Code = Code::new("shelf", "authors", "not_found");

/// Runs the codes check and returns its panic message, or `None` when it
/// passes.
fn problems(spelling: Spelling, codes: &[&[&Code]]) -> Option<String> {
    let outcome = panic::catch_unwind(|| check_codes(spelling, codes));
    let payload = outcome.err()?;
    Some(
        payload
            .downcast_ref::<String>()
            .cloned()
            .unwrap_or_default(),
    )
}

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

/// Spells a code with arrows between its segments.
fn arrows(code: &Code, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}→{}→{}", code.component(), code.area(), code.reason())
}

#[test]
fn distinct_codes_in_a_built_in_spelling_pass() {
    check_codes(
        Spelling::COLONS,
        &[&[&NOT_FOUND, &TIMED_OUT], &[&AUTHOR_NOT_FOUND]],
    );
}

#[test]
fn the_same_code_listed_by_two_error_types_passes() {
    check_codes(Spelling::DOTS, &[&[&NOT_FOUND], &[&NOT_FOUND, &TIMED_OUT]]);
}

#[test]
fn two_codes_that_spell_the_same_fail_and_both_are_named() {
    let message = problems(
        Spelling::custom(without_area),
        &[&[&NOT_FOUND], &[&AUTHOR_NOT_FOUND]],
    )
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
    let message = problems(Spelling::custom(empty), &[&[&NOT_FOUND]]).unwrap();

    assert!(
        message.contains(r#"Code::new("shelf", "books", "not_found") spells as an empty string"#),
        "{message}"
    );
}

#[test]
fn a_code_that_spells_with_whitespace_fails() {
    let message = problems(Spelling::custom(spaced), &[&[&NOT_FOUND]]).unwrap();

    assert!(
        message.contains(
            r#"Code::new("shelf", "books", "not_found") spells as "shelf books not_found", which holds whitespace"#
        ),
        "{message}"
    );
}

#[test]
fn a_code_that_spells_with_characters_outside_ascii_fails() {
    let message = problems(Spelling::custom(arrows), &[&[&NOT_FOUND]]).unwrap();

    assert!(
        message.contains(
            r#"Code::new("shelf", "books", "not_found") spells as "shelf→books→not_found", which holds characters outside ASCII"#
        ),
        "{message}"
    );
}

#[test]
fn one_failed_check_names_every_problem() {
    let message = problems(Spelling::custom(empty), &[&[&NOT_FOUND, &TIMED_OUT]]).unwrap();

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
