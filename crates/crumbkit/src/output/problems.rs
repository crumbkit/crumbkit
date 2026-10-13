//! Rows that report a problem through the field that `Row::PROBLEM` names,
//! which make a run without a failure exit with
//! [`exit::problem_rows`](crate::exit::problem_rows) (ADR-0050).

use serde_json::Value;

use crate::{Row, exit};

/// Returns the number of `rows` that report a problem. A row type that
/// names no problem field reports none, and its rows are not serialized.
pub(super) fn count<R: Row>(rows: &[R]) -> usize {
    if R::PROBLEM.is_none() {
        return 0;
    }
    rows.iter().filter(|row| reports(*row)).count()
}

/// Reports whether `row` reports a problem: whether the field that
/// `R::PROBLEM` names serializes to a value other than `null`, `false` or
/// an empty string.
///
/// A row that cannot be serialized reports no problem here, because the
/// write of the result reports the error and exits with 1.
pub(super) fn reports<R: Row>(row: &R) -> bool {
    let Some(field) = R::PROBLEM else {
        return false;
    };
    let Ok(value) = serde_json::to_value(row) else {
        return false;
    };
    match value.get(field) {
        None | Some(Value::Null | Value::Bool(false)) => false,
        Some(Value::String(text)) => !text.is_empty(),
        Some(_) => true,
    }
}

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
