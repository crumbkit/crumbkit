//! [`Problem`], the problem facet of a row type, which
//! [`Row::PROBLEM`](crate::Row::PROBLEM) holds (ADR-0096, ADR-0212).

use std::fmt;

use serde::Serialize;

use super::rule::Rule;

/// Which rows of the row type `R` report a problem, and the column that
/// shows it (ADR-0096, ADR-0212).
///
/// A run whose rows report a problem, and that has no failure, exits with 1,
/// and its text ends with a note that counts those rows. The exit reads every
/// row that the result holds. A listing from `Listing::from_all` also counts
/// the rows that its limit cut. A listing of one page that the tool fetched
/// counts only that page, so it puts the rows that report a problem first
/// (OUTPUT-3).
///
/// [`Problem::field`] names a field whose value marks a problem, and the
/// leaf's help prints a `Problems:` line that names it. [`Problem::by`]
/// takes a function that decides from other fields, such as a state enum,
/// in place of a second column that repeats the state as a flag, and names
/// the column that shows the state. Its help prints no `Problems:` line.
/// crumbkit-test's `check_help` fails either one when its column is not one
/// of `COLUMNS`.
///
/// ```
/// use crumbkit::{Problem, Row, RowReports};
/// use schemars::JsonSchema;
/// use serde::Serialize;
///
/// #[derive(Serialize, JsonSchema)]
/// #[serde(rename_all = "lowercase")]
/// enum State {
///     Done,
///     Failed,
/// }
///
/// #[derive(Serialize, JsonSchema)]
/// struct Job {
///     name: String,
///     state: State,
/// }
///
/// impl Job {
///     fn failed(&self) -> bool {
///         matches!(self.state, State::Failed)
///     }
/// }
///
/// impl Row for Job {
///     const COLUMNS: &'static [&'static str] = &["name", "state"];
///     const PROBLEM: Option<Problem<Self>> = Some(Problem::by("state", Self::failed));
/// }
///
/// let job = Job { name: "fetch".to_owned(), state: State::Failed };
/// assert!(job.reports_problem());
/// ```
// `Clone`, `Copy` and `Debug` are written by hand, because their derives
// would bound `R`, which a function pointer does not need. A function pointer
// has no reliable equality, so `Problem` has no `PartialEq`.
pub struct Problem<R: ?Sized>(Rule<R>);

impl<R: ?Sized> Problem<R> {
    /// Returns the problem of the field `field`, as the type serializes it,
    /// after any `#[serde(rename)]`. A row reports a problem when the field
    /// serializes to a value other than `null`, `false` or an empty string,
    /// and a missing field reports none.
    ///
    /// The leaf's help prints a `Problems:` line that names the field.
    /// crumbkit-test's `check_help` fails when the field is not one of
    /// `COLUMNS`, or when its type is not a string, a boolean or a value that
    /// can be null.
    #[must_use]
    pub const fn field(field: &'static str) -> Self {
        Self(Rule::field(field))
    }

    /// Returns the problem that `decides` reports for each row, shown in the
    /// column `column`, as the type serializes it.
    ///
    /// The leaf's help prints no `Problems:` line, and crumbkit-test's
    /// `check_help` fails when `column` is not one of `COLUMNS`, so the table
    /// shows the state that decides the exit.
    #[must_use]
    pub const fn by(column: &'static str, decides: fn(&R) -> bool) -> Self {
        Self(Rule::by(column, decides))
    }

    /// Returns the column that shows the problem.
    #[must_use]
    pub const fn get_column(self) -> &'static str {
        self.0.column()
    }

    /// Returns the field that [`Problem::field`] named, or `None` for
    /// [`Problem::by`].
    #[must_use]
    pub const fn get_field(self) -> Option<&'static str> {
        self.0.named_field()
    }
}

impl<R: Serialize + ?Sized> Problem<R> {
    /// Reports whether `row` reports a problem.
    ///
    /// For [`Problem::field`] it serializes the row. A row that cannot be
    /// serialized reports no problem, because the write of the result
    /// reports the error and exits with 1.
    #[must_use]
    pub fn reports(self, row: &R) -> bool {
        self.0.reports(row)
    }
}

impl<R: ?Sized> Clone for Problem<R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<R: ?Sized> Copy for Problem<R> {}

impl<R: ?Sized> fmt::Debug for Problem<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.debug("Problem", f)
    }
}

#[cfg(test)]
mod tests;
