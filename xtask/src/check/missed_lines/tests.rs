//! Tests for the missed line check.
//!
//! The tests build each `ADR-` title and citation with `format!`, so this
//! file contains no citation of its own for the repository check to find.

use super::check;
use crate::check::{File, Violation};

fn file(path: &str, contents: &str) -> File {
    File {
        path: path.to_owned(),
        contents: contents.to_owned(),
    }
}

/// Returns a record dated `date`.
fn record(number: u16, date: &str) -> File {
    file(
        &format!("docs/decisions/{number:04}-a.md"),
        &format!(
            "# ADR-{number:04}. A decision\n\n- Date: {date}\n- Status: accepted\n\n\
             ## Context\n\nThe context.\n"
        ),
    )
}

/// Returns a record dated `date` that amends `earlier`, with `missed` as
/// its Missed line when it has one.
fn amending(number: u16, date: &str, earlier: u16, missed: Option<&str>) -> File {
    let missed = missed.map_or_else(String::new, |text| format!("- Missed: {text}\n"));
    file(
        &format!("docs/decisions/{number:04}-b.md"),
        &format!(
            "# ADR-{number:04}. A later decision\n\n- Date: {date}\n- Status: accepted\n\
             - Supersedes: ADR-{earlier:04}, which it amends\n{missed}\n\
             ## Context\n\nThe context.\n"
        ),
    )
}

#[test]
fn a_new_record_that_amends_a_record_from_the_same_week_without_a_missed_line_is_reported() {
    let base = [record(1, "2026-10-14")];
    let head = [record(1, "2026-10-14"), amending(2, "2026-10-17", 1, None)];

    assert_eq!(
        check(&base, &head),
        [Violation::MissingMissedLine {
            path: "docs/decisions/0002-b.md".to_owned(),
            number: 2,
            earlier: 1,
        }]
    );
}

#[test]
fn a_new_record_that_says_what_the_earlier_record_missed_passes() {
    let base = [record(1, "2026-10-14")];
    let head = [
        record(1, "2026-10-14"),
        amending(
            2,
            "2026-10-17",
            1,
            Some("The earlier record left out batches."),
        ),
    ];

    assert_eq!(check(&base, &head), []);
}

#[test]
fn a_missed_line_without_text_is_reported() {
    let base = [record(1, "2026-10-14")];
    let head = [
        record(1, "2026-10-14"),
        amending(2, "2026-10-17", 1, Some("")),
    ];

    assert_eq!(check(&base, &head).len(), 1);
}

#[test]
fn a_new_record_that_amends_a_record_seven_days_older_needs_no_missed_line() {
    let base = [record(1, "2026-10-10")];
    let head = [record(1, "2026-10-10"), amending(2, "2026-10-17", 1, None)];

    assert_eq!(check(&base, &head), []);
}

#[test]
fn days_are_counted_across_the_end_of_a_month_and_a_year() {
    let base = [record(1, "2026-12-29"), record(3, "2026-09-28")];
    let head = [
        record(1, "2026-12-29"),
        record(3, "2026-09-28"),
        amending(2, "2027-01-02", 1, None),
        amending(4, "2026-10-02", 3, None),
    ];

    assert_eq!(check(&base, &head).len(), 2);
}

#[test]
fn a_record_that_is_already_at_the_merge_base_is_not_checked() {
    let base = [record(1, "2026-10-14"), amending(2, "2026-10-17", 1, None)];
    let head = [record(1, "2026-10-14"), amending(2, "2026-10-17", 1, None)];

    assert_eq!(check(&base, &head), []);
}

#[test]
fn a_history_without_records_at_the_merge_base_reports_nothing() {
    let head = [record(1, "2026-10-14"), amending(2, "2026-10-17", 1, None)];

    assert_eq!(check(&[], &head), []);
}
