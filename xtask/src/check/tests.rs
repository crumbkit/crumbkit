//! Tests for the shared check types.

use super::Violation;

#[test]
fn a_violation_prints_its_path_its_measure_and_the_limit_with_thousands_separators() {
    let violation = Violation {
        path: "docs".to_owned(),
        measured: 5312,
        unit: "words in this directory's Markdown",
        limit: 5000,
    };

    assert_eq!(
        violation.to_string(),
        "docs: 5,312 words in this directory's Markdown, limit 5,000"
    );
}

#[test]
fn numbers_below_1000_print_without_a_separator() {
    let violation = Violation {
        path: "AGENTS.md".to_owned(),
        measured: 401,
        unit: "words",
        limit: 400,
    };

    assert_eq!(violation.to_string(), "AGENTS.md: 401 words, limit 400");
}

#[test]
fn numbers_of_seven_digits_get_two_separators() {
    let violation = Violation {
        path: "src/lib.rs".to_owned(),
        measured: 1_234_567,
        unit: "lines",
        limit: 600,
    };

    assert_eq!(
        violation.to_string(),
        "src/lib.rs: 1,234,567 lines, limit 600"
    );
}
