//! The missed line check (DOCS-8). A record that is new on the branch and
//! names in its `- Supersedes:` line a record dated less than 7 days before
//! its own date needs a `- Missed:` line that says what the earlier record
//! missed. Records at the merge base are frozen, so the check reads only the
//! new ones. A history without records at the merge base, such as the empty
//! history of a test, makes the check pass, as it does the frozen record
//! check.

use std::collections::{BTreeMap, BTreeSet};

use super::adr_records::{citations, is_record, title_number};
use super::{File, Violation};

#[cfg(test)]
mod tests;

/// The number of days within which an amended record counts as being from
/// the same week as the record that amends it.
const SAME_WEEK: i64 = 7;

/// Reports each record in `head` that is not in `base`, amends a record
/// from the same week, and has no `- Missed:` line with text.
pub(super) fn check(base: &[File], head: &[File]) -> Vec<Violation> {
    let at_base: BTreeSet<u16> = records(base).map(|(number, _)| number).collect();
    if at_base.is_empty() {
        return Vec::new();
    }
    let dates: BTreeMap<u16, i64> = records(head)
        .filter_map(|(number, file)| Some((number, date(&file.contents)?)))
        .collect();

    let mut violations = Vec::new();
    for (number, file) in records(head).filter(|(number, _)| !at_base.contains(number)) {
        let Some(own) = dates.get(&number) else {
            continue;
        };
        if has_missed_line(&file.contents) {
            continue;
        }
        for earlier in supersedes(&file.contents) {
            let same_week = dates
                .get(&earlier)
                .is_some_and(|date| (own - date).abs() < SAME_WEEK);
            if same_week {
                violations.push(Violation::MissingMissedLine {
                    path: file.path.clone(),
                    number,
                    earlier,
                });
            }
        }
    }
    violations
}

/// Returns each record in `files` with its number.
fn records(files: &[File]) -> impl Iterator<Item = (u16, &File)> {
    files
        .iter()
        .filter(|file| is_record(&file.path))
        .filter_map(|file| Some((title_number(&file.contents)?, file)))
}

/// Returns the numbers that the record's `- Supersedes:` line cites.
fn supersedes(contents: &str) -> Vec<u16> {
    contents
        .lines()
        .filter(|line| line.starts_with("- Supersedes:"))
        .flat_map(citations)
        .collect()
}

/// Returns whether the record has a `- Missed:` line with text after it.
fn has_missed_line(contents: &str) -> bool {
    contents.lines().any(|line| {
        line.strip_prefix("- Missed:")
            .is_some_and(|text| !text.trim().is_empty())
    })
}

/// Returns the record's `- Date:` as a count of days since 1970-01-01, or
/// `None` when it has no date in the form `YYYY-MM-DD`.
fn date(contents: &str) -> Option<i64> {
    let text = contents
        .lines()
        .find_map(|line| line.strip_prefix("- Date: "))?;
    let mut parts = text.trim().splitn(3, '-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: i64 = parts.next()?.parse().ok()?;
    let day: i64 = parts.next()?.parse().ok()?;
    Some(days_from_civil(year, month, day))
}

/// Returns the number of days from 1970-01-01 to the given date of the
/// proleptic Gregorian calendar.
///
/// RUST-10: this is Howard Hinnant's `days_from_civil` algorithm, written
/// out because xtask depends only on clap and needs only the difference
/// between two record dates. It does not check that the day exists in the
/// month. Code that needs time zones or validation should use a date crate
/// such as jiff.
const fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_from_march = (month + 9) % 12;
    let day_of_year = (153 * month_from_march + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}
