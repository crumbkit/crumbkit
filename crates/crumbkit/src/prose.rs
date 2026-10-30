//! The result of a command whose answer is a few sentences.

use std::fmt;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Serialize, Serializer};

use crate::{AsFailure, Error, Failure, NoRow, Shape};

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
/// result with at least one failure exits with the exit code of its first
/// failure, which is 1 unless the tool's guide lists that failure's code
/// under one of its exits (ADR-0037). [`Prose::note`] adds a line that the
/// text prints on standard error before the failures, as a listing's note
/// prints.
///
/// `--fields`, and `--long` without `--json`, are refused for a prose
/// result, because it has no columns. `--long` or `--full` with `--json`
/// prints what `--json` alone prints, with a note that the flag adds
/// nothing (ADR-0149).
///
/// ```
/// use std::fmt;
///
/// use crumbkit::Prose;
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
    /// The line that the text output prints on standard error after the
    /// text, such as the path of the file that the answer comes from.
    note: Option<String>,
}

impl<T, E> Prose<T, E> {
    /// Returns a prose result of `value` without failures.
    #[must_use]
    pub const fn new(value: T) -> Self {
        Self {
            value,
            failures: Vec::new(),
            note: None,
        }
    }

    /// Returns a prose result of `value` and of the failures that the answer
    /// leaves out, which print in the order given.
    #[must_use]
    pub const fn with_failures(value: T, failures: Vec<E>) -> Self {
        Self {
            value,
            failures,
            note: None,
        }
    }

    /// Sets the note: a line that the text output prints on standard error
    /// after the text and before any failure, such as `Read from
    /// /home/ada/books.toml`. The text carries its
    /// own full stop when it needs one. The JSON document leaves it out, as
    /// it leaves out a listing's note ([`Listing::note`](crate::Listing::note)).
    #[must_use]
    pub fn note(mut self, text: impl Into<String>) -> Self {
        self.note = Some(text.into());
        self
    }

    /// Returns the value whose `Display` the text output prints. A tool's
    /// test reads the answer of the prose result that its handler returned
    /// here.
    #[must_use]
    pub const fn value(&self) -> &T {
        &self.value
    }

    /// Returns the failures that the result holds, in order.
    #[must_use]
    pub fn failures(&self) -> &[E] {
        &self.failures
    }

    /// Returns the note that [`Prose::note`] set, or `None` when the tool
    /// set none. The getter has the `get_` prefix because the builder
    /// takes the plain name, as `Guide` and `Leaf` getters do.
    #[must_use]
    pub fn get_note(&self) -> Option<&str> {
        self.note.as_deref()
    }
}

/// Serializes the prose result as its value. `crumbkit::finish` writes the
/// failures beside it, under `failures`.
impl<T: Serialize, E> Serialize for Prose<T, E> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.value.serialize(serializer)
    }
}

impl<T: Serialize + JsonSchema + fmt::Display, E: AsFailure> crate::row::Sealed for Prose<T, E> {
    type Row = NoRow;
    const SHAPE: Shape = Shape::Prose;
    type Summary = crate::NoSummary;

    fn rows(&self) -> &[NoRow] {
        &[]
    }

    fn failures(&self) -> Option<Vec<&dyn Failure>> {
        Some(self.failures.iter().map(AsFailure::as_failure).collect())
    }

    fn note(&self) -> Option<&str> {
        self.get_note()
    }

    fn prose(&self) -> Option<&dyn fmt::Display> {
        Some(self.value())
    }

    fn prose_schema(generator: &mut SchemaGenerator) -> Option<Schema> {
        Some(generator.subschema_for::<T>())
    }
}
