//! The note of the rows that report a problem, as `Row::is_problem`
//! decides, which make a run without a failure exit with
//! [`exit::problem_rows`](crate::exit::problem_rows) (ADR-0050), and the
//! rows that the default text of a listing prints, which `Row::in_default_text`
//! and `Document::only_problems` choose (ADR-0138, ADR-0157). The
//! `row::problems` module decides which rows report one.

use crate::view::{Table, View};
use crate::{Document, Row, Rows, Shape, exit};

/// The rule that picks the rows that the default text of a listing lists
/// (ADR-0138, ADR-0157): each row that reports a problem, and each other
/// row whose `Row::in_default_text` is true unless the document lists only
/// the rows that report a problem.
#[derive(Clone, Copy)]
pub(super) struct DefaultText {
    /// Whether the document is marked with [`Document::only_problems`].
    only_problems: bool,
}

impl DefaultText {
    /// Returns the rule of `document` in `view`, or `None` when the text
    /// lists every row: the result is not a listing, or `view` is not the
    /// compact text table, because the caller passed `--json`, `--long` or
    /// `--fields`.
    pub(super) fn of<T: Rows>(document: &Document<T>, view: &View) -> Option<Self> {
        (T::SHAPE == Shape::Listing && *view == View::Text(Table::Compact)).then(|| Self {
            only_problems: document.lists_only_problems(),
        })
    }

    /// Reports whether the default text lists `row`. `Row::in_default_text`
    /// is asked first, because its default answers without the work that
    /// `Row::is_problem` can do, such as serializing the problem field.
    fn lists<R: Row>(self, row: &R) -> bool {
        if self.only_problems {
            row.is_problem()
        } else {
            row.in_default_text() || row.is_problem()
        }
    }
}

/// Returns the rows of `rows` that the text prints: the rows that `rule`
/// lists, or every row when there is no rule.
pub(super) fn shown<R: Row>(rows: &[R], rule: Option<DefaultText>) -> impl Iterator<Item = &R> {
    rows.iter()
        .filter(move |row| rule.is_none_or(|rule| rule.lists(*row)))
}

/// Reports whether a text that prints `shown` of the `held` rows of a
/// result under `rule` leaves rows out, so that a summary line or nothing
/// stands in place of the header when it prints none (ADR-0153). A document
/// that lists only the rows that report a problem counts as leaving rows
/// out even when it holds none, as ADR-0138 decided.
pub(super) fn leaves_out(rule: Option<DefaultText>, shown: usize, held: usize) -> bool {
    rule.is_some_and(|rule| rule.only_problems || shown < held)
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
