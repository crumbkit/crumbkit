//! The one-call checks for whether a row reports a problem or a warning
//! (ADR-0212, ADR-0245).

use crate::Row;

/// Asks a row whether it reports a problem or a warning.
///
/// Every [`Row`] has this method through a blanket impl, so no row type can
/// override the answer and disagree with [`Row::PROBLEM`]. Import the trait
/// to call the method.
///
/// ```
/// use crumbkit::{Problem, Row, RowReports};
/// use schemars::JsonSchema;
/// use serde::Serialize;
///
/// #[derive(Serialize, JsonSchema)]
/// struct Job {
///     /// The job's name.
///     name: String,
///     /// Whether the job failed.
///     failed: bool,
/// }
///
/// impl Row for Job {
///     const COLUMNS: &'static [&'static str] = &["name", "failed"];
///     const PROBLEM: Option<Problem<Self>> = Some(Problem::field("failed"));
/// }
///
/// let job = Job { name: "fetch".to_owned(), failed: true };
/// assert!(job.reports_problem());
/// ```
pub trait RowReports: Row {
    /// Reports whether the row reports a problem, as [`Row::PROBLEM`] decides.
    /// A row type that names no problem reports none.
    #[must_use]
    fn reports_problem(&self) -> bool;

    /// Reports whether the row reports a warning, as [`Row::WARNING`]
    /// decides, and no problem, so a row counts once, as its worse state. A
    /// row type that names no warning reports none.
    #[must_use]
    fn reports_warning(&self) -> bool;
}

impl<R: Row + ?Sized> RowReports for R {
    fn reports_problem(&self) -> bool {
        R::PROBLEM.is_some_and(|problem| problem.reports(self))
    }

    fn reports_warning(&self) -> bool {
        R::WARNING.is_some_and(|warning| warning.reports(self)) && !self.reports_problem()
    }
}

#[cfg(test)]
mod tests;
