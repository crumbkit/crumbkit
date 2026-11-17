//! [`Warning`], the warning facet of a row type, which
//! [`Row::WARNING`](crate::Row::WARNING) holds (ADR-0245).

use std::fmt;

use serde::Serialize;

use super::rule::Rule;

/// Which rows of the row type `R` report a warning, and the column that
/// shows it (ADR-0245).
///
/// A warning marks a row that deserves attention and does not change the
/// exit code. A run whose rows report only warnings exits with 0, and its
/// text ends with a note that counts those rows. A row that reports a
/// problem and a warning counts only as a problem. The default text lists a
/// row that reports a warning whatever `Row::in_default_text` returns, and
/// a listing lists such rows after the rows that report a problem (OUTPUT-3).
///
/// [`Warning::field`] names a field whose value marks a warning, and the
/// leaf's help prints a `Warnings:` line that names it. [`Warning::by`]
/// takes a function that decides, and prints no such line, as
/// [`Problem::by`](crate::Problem::by) prints none. crumbkit-test's
/// `check_help` fails either one when its column is not one of `COLUMNS`.
///
/// ```
/// use crumbkit::{Row, RowReports, Warning};
/// use schemars::JsonSchema;
/// use serde::Serialize;
///
/// #[derive(Serialize, JsonSchema)]
/// struct Job {
///     name: String,
///     retried: bool,
/// }
///
/// impl Job {
///     fn retried(&self) -> bool {
///         self.retried
///     }
/// }
///
/// impl Row for Job {
///     const COLUMNS: &'static [&'static str] = &["name", "retried"];
///     const WARNING: Option<Warning<Self>> = Some(Warning::by("retried", Self::retried));
/// }
///
/// let job = Job { name: "fetch".to_owned(), retried: true };
/// assert!(job.reports_warning());
/// ```
// `Clone`, `Copy` and `Debug` are written by hand, because their derives
// would bound `R`, which a function pointer does not need. A function pointer
// has no reliable equality, so `Warning` has no `PartialEq`.
pub struct Warning<R: ?Sized>(Rule<R>);

impl<R: ?Sized> Warning<R> {
    /// Returns the warning of the field `field`, as the type serializes it,
    /// after any `#[serde(rename)]`. A row reports a warning when the field
    /// serializes to a value other than `null`, `false` or an empty string,
    /// and a missing field reports none.
    ///
    /// The leaf's help prints a `Warnings:` line that names the field.
    /// crumbkit-test's `check_help` fails when the field is not one of
    /// `COLUMNS`, when its type is not a string, a boolean or a value that
    /// can be null, or when it is the field of `Row::PROBLEM`.
    #[must_use]
    pub const fn field(field: &'static str) -> Self {
        Self(Rule::field(field))
    }

    /// Returns the warning that `decides` reports for each row, shown in the
    /// column `column`, as the type serializes it.
    ///
    /// The leaf's help prints no `Warnings:` line, and crumbkit-test's
    /// `check_help` fails when `column` is not one of `COLUMNS`.
    #[must_use]
    pub const fn by(column: &'static str, decides: fn(&R) -> bool) -> Self {
        Self(Rule::by(column, decides))
    }

    /// Returns the column that shows the warning.
    #[must_use]
    pub const fn get_column(self) -> &'static str {
        self.0.column()
    }

    /// Returns the field that [`Warning::field`] named, or `None` for
    /// [`Warning::by`].
    #[must_use]
    pub const fn get_field(self) -> Option<&'static str> {
        self.0.named_field()
    }
}

impl<R: Serialize + ?Sized> Warning<R> {
    /// Reports whether `row` reports a warning, whether or not it also
    /// reports a problem.
    ///
    /// For [`Warning::field`] it serializes the row. A row that cannot be
    /// serialized reports no warning, because the write of the result
    /// reports the error and exits with 1.
    #[must_use]
    pub fn reports(self, row: &R) -> bool {
        self.0.reports(row)
    }
}

impl<R: ?Sized> Clone for Warning<R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<R: ?Sized> Copy for Warning<R> {}

impl<R: ?Sized> fmt::Debug for Warning<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.debug("Warning", f)
    }
}

#[cfg(test)]
mod tests;
