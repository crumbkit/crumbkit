//! The note of the rows that report a problem, as `Row::is_problem`
//! decides, which make a run without a failure exit with
//! [`exit::problem_rows`](crate::exit::problem_rows) (ADR-0050), and the
//! rows that the text of a document that lists only those rows prints
//! (ADR-0138). The `row::problems` module decides which rows report one.

use crate::view::{Table, View};
use crate::{Document, Row, exit};

/// Reports whether the text of `document` in `view` lists only the rows
/// that report a problem: the document is marked with
/// [`Document::only_problems`], and `view` is the compact text table, so
/// the caller passed no `--json`, `--long` or `--fields`.
pub(super) fn only<T>(document: &Document<T>, view: &View) -> bool {
    document.lists_only_problems() && *view == View::Text(Table::Compact)
}

/// Returns the rows of `rows` that the text prints: each row that reports a
/// problem when `only` is set, and every row otherwise.
pub(super) fn shown<R: Row>(rows: &[R], only: bool) -> impl Iterator<Item = &R> {
    rows.iter().filter(move |row| !only || row.is_problem())
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
