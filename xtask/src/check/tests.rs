//! Tests for the shared check types.

use super::Violation;

#[test]
fn a_violation_prints_its_path_its_measure_and_the_limit_with_thousands_separators() {
    let violation = Violation::OverLimit {
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
    let violation = Violation::OverLimit {
        path: "AGENTS.md".to_owned(),
        measured: 201,
        unit: "words",
        limit: 200,
    };

    assert_eq!(violation.to_string(), "AGENTS.md: 201 words, limit 200");
}

#[test]
fn numbers_of_seven_digits_get_two_separators() {
    let violation = Violation::OverLimit {
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

#[test]
fn record_violations_name_the_file_and_the_number() {
    let duplicate = Violation::DuplicateRecord {
        path: "docs/decisions/b.md".to_owned(),
        number: 3,
        first: "docs/decisions/a.md".to_owned(),
    };
    let unknown = Violation::UnknownRecord {
        path: "AGENTS.md".to_owned(),
        number: 99,
    };

    assert_eq!(
        duplicate.to_string(),
        format!(
            "docs/decisions/b.md: ADR-{:04} is already used by docs/decisions/a.md",
            3
        )
    );
    assert_eq!(
        unknown.to_string(),
        format!("AGENTS.md: cites ADR-{:04}, which no record uses", 99)
    );
}
