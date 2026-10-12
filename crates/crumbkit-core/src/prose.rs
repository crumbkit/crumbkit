//! The result of a command whose answer is a few sentences.

use std::fmt;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Serialize, Serializer};

use crate::{AsFailure, Error, Failure, NoRow, Rows, Shape};

#[cfg(test)]
mod tests;

/// The result of a command whose answer is a few sentences: a value that
/// prints as text with its `Display`, and that the JSON document holds under
/// `data` with its `Serialize` and describes with its `JsonSchema`. It may
/// also hold failures, such as one for each id the answer could not cover.
///
/// `crumbkit::finish` prints the text on standard output and ends it with a
/// line feed when it lacks one. It escapes each control character other
/// than a line feed or a tab, as the table escapes a cell, and it prints
/// nothing for an empty text. With `--json`, the document holds the value
/// under `data` and the failures under `failures`, which is present even
/// when the list is empty, as a batch's is. In text, each failure prints on
/// standard error after the text, as a batch prints its failures. A prose
/// result with at least one failure exits with code 1.
///
/// `--fields` is refused for a prose result, because it has no columns, and
/// `--long` and `--full` leave its output unchanged.
///
/// ```
/// use std::fmt;
///
/// use crumbkit_core::Prose;
///
/// /// What the catalogue holds.
/// #[derive(serde::Serialize, schemars::JsonSchema)]
/// struct Summary {
///     /// The number of books.
///     books: u32,
///     /// The number of authors.
///     authors: u32,
/// }
///
/// impl fmt::Display for Summary {
///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         write!(f, "{} books by {} authors.", self.books, self.authors)
///     }
/// }
///
/// let prose: Prose<Summary> = Prose::new(Summary { books: 6, authors: 6 });
/// # let _ = prose;
/// ```
///
/// The failure type `E` is crumbkit's [`Error`] unless the tool names its
/// own, as in `Prose<Summary, BooksError>`.
#[derive(Debug)]
pub struct Prose<T, E = Error> {
    /// The answer, which prints as text and serializes under `data`.
    value: T,
    /// The failures that the answer leaves out, in the order they print.
    failures: Vec<E>,
}

impl<T, E> Prose<T, E> {
    /// Returns a prose result of `value` without failures.
    #[must_use]
    pub const fn new(value: T) -> Self {
        Self {
            value,
            failures: Vec::new(),
        }
    }

    /// Returns a prose result of `value` and of the failures that the answer
    /// leaves out, which print in the order given.
    #[must_use]
    pub const fn with_failures(value: T, failures: Vec<E>) -> Self {
        Self { value, failures }
    }
}

/// Serializes the prose result as its value. `crumbkit::finish` writes the
/// failures beside it, under `failures`.
impl<T: Serialize, E> Serialize for Prose<T, E> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.value.serialize(serializer)
    }
}

impl<T: Serialize + JsonSchema + fmt::Display, E: AsFailure> Rows for Prose<T, E> {
    type Row = NoRow;
    const SHAPE: Shape = Shape::Prose;
    type Summary = crate::NoSummary;

    fn rows(&self) -> &[NoRow] {
        &[]
    }

    fn failures(&self) -> Option<Vec<&dyn Failure>> {
        Some(self.failures.iter().map(AsFailure::as_failure).collect())
    }

    fn prose(&self) -> Option<&dyn fmt::Display> {
        Some(&self.value)
    }

    fn prose_schema(generator: &mut SchemaGenerator) -> Option<Schema> {
        Some(generator.subschema_for::<T>())
    }
}
