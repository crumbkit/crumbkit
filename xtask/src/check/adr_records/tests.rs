//! Tests for the decision record check.
//!
//! The tests build each `ADR-` citation with `format!`, so this file contains
//! no citation of its own for the repository check to find.

use super::check;
use crate::check::{File, Violation};

fn file(path: &str, contents: &str) -> File {
    File {
        path: path.to_owned(),
        contents: contents.to_owned(),
    }
}

fn record(path: &str, number: u16) -> File {
    file(
        path,
        &format!("# ADR-{number:04}. A decision\n\nThe text.\n"),
    )
}

fn cite(number: u16) -> String {
    format!("See ADR-{number:04} for the reason.\n")
}

#[test]
fn records_with_distinct_numbers_and_citations_of_them_pass() {
    let files = [
        record("docs/decisions/README.md", 17),
        record("docs/decisions/api/0001-errors.md", 1),
        file("AGENTS.md", &cite(1)),
        file("xtask/src/main.rs", &format!("// {}", cite(17))),
    ];

    assert_eq!(check(&files), []);
}

#[test]
fn a_citation_of_a_number_no_record_defines_is_reported() {
    let files = [
        record("docs/decisions/0001-errors.md", 1),
        file("AGENTS.md", &cite(2)),
    ];

    assert_eq!(
        check(&files),
        [Violation::UnknownRecord {
            path: "AGENTS.md".to_owned(),
            number: 2,
        }]
    );
}

#[test]
fn a_record_that_cites_itself_passes() {
    let files = [file(
        "docs/decisions/0001-errors.md",
        &format!("# ADR-0001. Errors\n\n{}", cite(1)),
    )];

    assert_eq!(check(&files), []);
}

#[test]
fn two_records_with_one_number_are_reported_at_the_second() {
    let files = [
        record("docs/decisions/api/0003-a.md", 3),
        record("docs/decisions/project/0003-b.md", 3),
    ];

    assert_eq!(
        check(&files),
        [Violation::DuplicateRecord {
            path: "docs/decisions/project/0003-b.md".to_owned(),
            number: 3,
            first: "docs/decisions/api/0003-a.md".to_owned(),
        }]
    );
}

#[test]
fn a_decisions_file_without_a_record_title_is_reported() {
    let files = [
        file("docs/decisions/README.md", "# Decision records\n"),
        file("docs/decisions/b.md", &format!("# ADR-{:03}. Short\n", 5)),
    ];

    assert_eq!(
        check(&files),
        [
            Violation::MissingRecordTitle {
                path: "docs/decisions/README.md".to_owned(),
            },
            Violation::MissingRecordTitle {
                path: "docs/decisions/b.md".to_owned(),
            },
        ]
    );
}

#[test]
fn text_that_only_resembles_a_citation_is_ignored() {
    let files = [file(
        "AGENTS.md",
        &format!("ADR-NNNN, ADR-{:03}, ADR-{:05} and ADR {:04}\n", 1, 2, 3),
    )];

    assert_eq!(check(&files), []);
}

#[test]
fn files_outside_the_decisions_tree_need_no_record_title() {
    let files = [
        file("docs/standards/README.md", "# Standards\n"),
        file("docs/decisions.md", "# Decisions\n"),
    ];

    assert_eq!(check(&files), []);
}
