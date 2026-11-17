//! Rows that report a problem or a warning, as `Row::PROBLEM` and
//! `Row::WARNING` decide (ADR-0050, ADR-0096, ADR-0212, ADR-0244). `finish`
//! and a stream read these counts to choose the exit code and word the
//! notes.

use crate::{Row, RowReports};

/// The numbers of rows that report a problem and a warning. A row that
/// reports both counts as a problem only. It is `pub` inside the crate-private
/// module `row::problems`, because the sealed `Rows` supertrait names it, and
/// no path outside crumbkit reaches it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Marks {
    /// The rows that report a problem, which decide the exit code.
    pub(crate) problems: usize,
    /// The rows that report a warning and no problem.
    pub(crate) warnings: usize,
}

impl Marks {
    /// Counts `row` in the marks.
    pub(crate) fn add<R: Row + ?Sized>(&mut self, row: &R) {
        if row.reports_problem() {
            self.problems += 1;
        } else if row.reports_warning() {
            self.warnings += 1;
        }
    }

    /// Reports whether no row is marked.
    pub(crate) const fn is_empty(self) -> bool {
        self.problems == 0 && self.warnings == 0
    }
}

/// Returns the marks of `rows`, as [`Row::PROBLEM`] and [`Row::WARNING`]
/// decide. A row type that names neither reports none, and its rows are not
/// serialized.
#[must_use]
pub(crate) fn count<R: Row>(rows: &[R]) -> Marks {
    let mut marks = Marks::default();
    for row in rows {
        marks.add(row);
    }
    marks
}
