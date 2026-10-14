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
fn an_edited_record_names_the_lines_that_may_change() {
    let violation = Violation::EditedRecord {
        path: "docs/decisions/0001-a.md".to_owned(),
        number: 1,
    };

    assert_eq!(
        violation.to_string(),
        format!(
            "docs/decisions/0001-a.md: edits ADR-{:04}, which is frozen; only its Status \
             line and a node's Children list may change (DOCS-6)",
            1
        )
    );
}

#[test]
fn a_superseded_citation_names_the_rule() {
    let violation = Violation::SupersededCitation {
        path: "README.md".to_owned(),
        number: 1,
    };

    assert_eq!(
        violation.to_string(),
        format!(
            "README.md: cites ADR-{:04}, which is superseded; cite the page in docs/ \
             that describes the behaviour, or the record that superseded it",
            1
        )
    );
}

#[test]
fn a_missing_docs_trailer_shows_both_trailer_forms() {
    let violation = Violation::MissingDocsTrailer {
        path: "CHANGELOG.md".to_owned(),
    };

    assert_eq!(
        violation.to_string(),
        "CHANGELOG.md: an Unreleased entry changed, so a commit message must end with \
         `Docs-Owed: <id>; from: <old behaviour>; to: <new behaviour>; docs: <path>[, <path>...]` \
         or `Docs-Swept: <id>[, <id>...]` (DOCS-7)"
    );
}

#[test]
fn a_malformed_trailer_names_its_commit_and_shows_both_forms() {
    let violation = Violation::MalformedTrailer {
        commit: "abc1234".to_owned(),
        trailer: "Docs-Swept: Bad Id".to_owned(),
    };

    assert_eq!(
        violation.to_string(),
        "commit abc1234: `Docs-Swept: Bad Id` does not parse; write \
         `Docs-Owed: <id>; from: <old behaviour>; to: <new behaviour>; docs: <path>[, <path>...]` \
         or `Docs-Swept: <id>[, <id>...]`, where <id> is a kebab-case slug (DOCS-7)"
    );
}
