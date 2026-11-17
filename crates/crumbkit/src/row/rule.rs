//! How a row type decides which rows are marked, shared by [`Problem`] and
//! [`Warning`] (ADR-0212, ADR-0245).
//!
//! [`Problem`]: crate::Problem
//! [`Warning`]: crate::Warning

use std::fmt;

use serde::Serialize;
use serde_json::Value;

/// The column that shows a mark and how a row is decided to carry it.
pub(super) struct Rule<R: ?Sized> {
    /// The column that shows the mark, as the type serializes it.
    column: &'static str,
    /// How a row is decided to carry the mark.
    how: How<R>,
}

/// How a [`Rule`] decides whether a row is marked.
enum How<R: ?Sized> {
    /// The value of the column's field decides.
    Field,
    /// The function decides.
    By(fn(&R) -> bool),
}

impl<R: ?Sized> Rule<R> {
    /// Returns the rule that the value of `field` decides.
    pub(super) const fn field(field: &'static str) -> Self {
        Self {
            column: field,
            how: How::Field,
        }
    }

    /// Returns the rule that `decides`, shown in `column`.
    pub(super) const fn by(column: &'static str, decides: fn(&R) -> bool) -> Self {
        Self {
            column,
            how: How::By(decides),
        }
    }

    /// Returns the column that shows the mark.
    pub(super) const fn column(self) -> &'static str {
        self.column
    }

    /// Returns the field that [`Rule::field`] named, or `None`.
    pub(super) const fn named_field(self) -> Option<&'static str> {
        match self.how {
            How::Field => Some(self.column),
            How::By(_) => None,
        }
    }

    /// Writes the rule as the struct `name` of a `Debug` output.
    pub(super) fn debug(self, name: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let how = match self.how {
            How::Field => "Field",
            How::By(_) => "By",
        };
        f.debug_struct(name)
            .field("column", &self.column)
            .field("rule", &format_args!("{how}"))
            .finish()
    }
}

impl<R: Serialize + ?Sized> Rule<R> {
    /// Reports whether `row` is marked.
    ///
    /// For a field rule it serializes the row. A row that cannot be
    /// serialized is not marked, because the write of the result reports the
    /// error and exits with 1.
    pub(super) fn reports(self, row: &R) -> bool {
        match self.how {
            How::By(decides) => decides(row),
            How::Field => {
                let Ok(value) = serde_json::to_value(row) else {
                    return false;
                };
                match value.get(self.column) {
                    None | Some(Value::Null | Value::Bool(false)) => false,
                    Some(Value::String(text)) => !text.is_empty(),
                    Some(_) => true,
                }
            }
        }
    }
}

impl<R: ?Sized> Clone for Rule<R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<R: ?Sized> Copy for Rule<R> {}

impl<R: ?Sized> Clone for How<R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<R: ?Sized> Copy for How<R> {}
