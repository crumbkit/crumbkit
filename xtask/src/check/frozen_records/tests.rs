//! Tests for the frozen record check.
//!
//! The tests build each `ADR-` title with `format!`, so this file contains no
//! citation of its own for the repository check to find.

use super::check;
use crate::check::{File, Violation};

fn file(path: &str, contents: &str) -> File {
    File {
        path: path.to_owned(),
        contents: contents.to_owned(),
    }
}

/// Returns a dated record with the given status and decision text.
fn dated(number: u16, status: &str, decision: &str) -> String {
    format!(
        "# ADR-{number:04}. A decision\n\n- Date: 2026-10-14\n- Status: {status}\n\n\
         ## Context\n\nThe context.\n\n## Decision\n\n{decision}\n\nConsidered: another way.\n"
    )
}

/// Returns a dated node record with the given children list.
fn node(number: u16, children: &str) -> String {
    format!(
        "# ADR-{number:04}. A topic\n\n- Date: 2026-10-14\n- Status: accepted\n\n\
         ## Decision\n\nThese records decided a topic.\n\n## Children\n\n{children}"
    )
}

#[test]
fn an_unchanged_dated_record_passes() {
    let base = [file(
        "docs/decisions/0001-a.md",
        &dated(1, "accepted", "A."),
    )];
    let head = [file(
        "docs/decisions/0001-a.md",
        &dated(1, "accepted", "A."),
    )];

    assert_eq!(check(&base, &head), []);
}

#[test]
fn an_edit_to_the_decision_of_a_dated_record_is_reported() {
    let base = [file(
        "docs/decisions/0001-a.md",
        &dated(1, "accepted", "A."),
    )];
    let head = [file(
        "docs/decisions/0001-a.md",
        &dated(1, "accepted", "B."),
    )];

    assert_eq!(
        check(&base, &head),
        [Violation::EditedRecord {
            path: "docs/decisions/0001-a.md".to_owned(),
            number: 1,
        }]
    );
}

#[test]
fn a_changed_status_line_passes() {
    let superseded = format!("superseded by ADR-{:04}", 2);
    let base = [file(
        "docs/decisions/0001-a.md",
        &dated(1, "accepted", "A."),
    )];
    let head = [file(
        "docs/decisions/0001-a.md",
        &dated(1, &superseded, "A."),
    )];

    assert_eq!(check(&base, &head), []);
}

#[test]
fn a_changed_children_list_of_a_node_passes() {
    let base = [file(
        "docs/decisions/README.md",
        &node(17, "- [One](0001-a.md)\n"),
    )];
    let head = [file(
        "docs/decisions/README.md",
        &node(17, "- [One](0001-a.md)\n- [Two](0002-b.md)\n"),
    )];

    assert_eq!(check(&base, &head), []);
}

#[test]
fn an_edit_after_the_children_list_is_reported() {
    let base = [file(
        "docs/decisions/README.md",
        &node(17, "- [One](0001-a.md)\n"),
    )];
    let edited = format!(
        "{}\n## Notes\n\nA new note.\n",
        node(17, "- [One](0001-a.md)\n")
    );
    let head = [file("docs/decisions/README.md", &edited)];

    assert_eq!(
        check(&base, &head),
        [Violation::EditedRecord {
            path: "docs/decisions/README.md".to_owned(),
            number: 17,
        }]
    );
}

#[test]
fn a_dated_record_that_moved_to_another_directory_passes() {
    let base = [file(
        "docs/decisions/0001-a.md",
        &dated(1, "accepted", "A."),
    )];
    let head = [file(
        "docs/decisions/api/0001-a.md",
        &dated(1, "accepted", "A."),
    )];

    assert_eq!(check(&base, &head), []);
}

#[test]
fn a_removed_dated_record_is_reported_at_its_old_path() {
    let base = [file(
        "docs/decisions/0001-a.md",
        &dated(1, "accepted", "A."),
    )];

    assert_eq!(
        check(&base, &[]),
        [Violation::EditedRecord {
            path: "docs/decisions/0001-a.md".to_owned(),
            number: 1,
        }]
    );
}

#[test]
fn a_record_without_a_date_line_in_the_base_may_be_rewritten() {
    let undated = format!("# ADR-{:04}. A decision\n\n- The old text.\n", 1);
    let base = [file("docs/decisions/0001-a.md", &undated)];
    let head = [file(
        "docs/decisions/0001-a.md",
        &dated(1, "accepted", "A."),
    )];

    assert_eq!(check(&base, &head), []);
}

#[test]
fn a_new_record_passes() {
    let head = [file(
        "docs/decisions/0002-b.md",
        &dated(2, "accepted", "B."),
    )];

    assert_eq!(check(&[], &head), []);
}

#[test]
fn files_outside_the_decisions_tree_are_not_frozen() {
    let base = [file(
        "docs/behaviour/README.md",
        "- Date: 2026-10-14\nOld.\n",
    )];
    let head = [file(
        "docs/behaviour/README.md",
        "- Date: 2026-10-14\nNew.\n",
    )];

    assert_eq!(check(&base, &head), []);
}
