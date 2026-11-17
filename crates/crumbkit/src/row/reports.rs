//! The one-call check for whether a row reports a problem (ADR-0212).

use crate::Row;

/// Asks a row whether it reports a problem.
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
}

impl<R: Row + ?Sized> RowReports for R {
    fn reports_problem(&self) -> bool {
        R::PROBLEM.is_some_and(|problem| problem.reports(self))
    }
}

#[cfg(test)]
mod tests;
