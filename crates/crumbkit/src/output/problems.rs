//! The note of the rows that report a problem through the field that
//! `Row::PROBLEM` names, which make a run without a failure exit with
//! [`exit::problem_rows`](crate::exit::problem_rows) (ADR-0050).
//! crumbkit-core's `problems` module decides which rows report one.

use crate::exit;

/// Returns the note that says how many rows report a problem and the exit
/// code that follows, such as `note: 2 of the aliases report a problem, so
/// the command exits with 1.`, with `rows` when there is no description.
/// The text prints it on standard error when `problems` is more than 0 and
/// the run has no failure.
pub(super) fn note(problems: usize, description: Option<&str>) -> String {
    let description = description.unwrap_or("rows");
    let verb = if problems == 1 { "reports" } else { "report" };
    let exit = exit::problem_rows();
    format!(
        "note: {problems} of the {description} {verb} a problem, so the command exits with {exit}."
    )
}
