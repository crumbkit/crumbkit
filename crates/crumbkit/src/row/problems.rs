//! Rows that report a problem, as `Row::PROBLEM` decides (ADR-0050,
//! ADR-0096, ADR-0212). `finish` and a stream read this function to choose
//! the exit code and word the note.

use crate::{Row, RowReports};

/// Returns the number of `rows` that report a problem, as [`Row::PROBLEM`]
/// decides. A row type that names no problem reports none, and its rows are
/// not serialized.
#[must_use]
pub(crate) fn count<R: Row>(rows: &[R]) -> usize {
    rows.iter().filter(|row| row.reports_problem()).count()
}
