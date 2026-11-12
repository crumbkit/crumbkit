//! Rows that report a problem, as `Row::PROBLEM` decides (ADR-0050,
//! ADR-0096, ADR-0212). `finish` and a stream read these functions to choose
//! the exit code and word the note.

use crate::Row;

/// Returns the number of `rows` that report a problem, as [`Row::PROBLEM`]
/// decides. A row type that names no problem reports none, and its rows are
/// not serialized.
#[must_use]
pub(crate) fn count<R: Row>(rows: &[R]) -> usize {
    rows.iter().filter(|row| reports(*row)).count()
}

/// Reports whether `row` reports a problem, as [`Row::PROBLEM`] decides. A
/// row type that names no problem reports none.
#[must_use]
pub(crate) fn reports<R: Row + ?Sized>(row: &R) -> bool {
    R::PROBLEM.is_some_and(|problem| problem.reports(row))
}
