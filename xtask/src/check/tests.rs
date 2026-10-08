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

#[test]
fn listing_violations_name_the_node_or_the_link() {
    let unlisted = Violation::UnlistedRecord {
        path: "docs/decisions/api/0001-errors.md".to_owned(),
        node: "docs/decisions/api/README.md".to_owned(),
    };
    let broken = Violation::BrokenLink {
        path: "docs/decisions/README.md".to_owned(),
        target: "0009-gone.md".to_owned(),
    };

    assert_eq!(
        unlisted.to_string(),
        "docs/decisions/api/0001-errors.md: not listed in docs/decisions/api/README.md"
    );
    assert_eq!(
        broken.to_string(),
        "docs/decisions/README.md: links to 0009-gone.md, which does not exist"
    );
}

#[test]
fn a_property_test_violation_names_the_file_the_line_and_the_required_attribute() {
    let violation = Violation::UnconfiguredPropertyTests {
        path: "crates/crumbkit/src/code/tests.rs".to_owned(),
        line: 104,
    };

    assert_eq!(
        violation.to_string(),
        "crates/crumbkit/src/code/tests.rs:104: a proptest! block must start with \
         #![proptest_config(crate::property::config())]"
    );
}
