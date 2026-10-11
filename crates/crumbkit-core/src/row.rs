//! The traits that let a result print as a table.

use schemars::JsonSchema;
use serde::Serialize;

use crate::{Call, Failure};

/// A type that prints as one table row.
///
/// The names in `COLUMNS` and `LONG_COLUMNS` are the field names as the type
/// serializes them, after any `#[serde(rename)]`. The fields of a row type
/// are the properties of its JSON Schema. When the type is an enum whose
/// schema lists its variants under `oneOf` or `anyOf`, they also include the
/// properties of each variant, so `--fields` and a leaf's columns can name a
/// field that only some variants have. A name that is not a field
/// prints `-` in every row of its column, and `null` in JSON. crumbkit-test's
/// `check_guide` fails on such a name in the row type of each leaf that the
/// tool's guide gives a document, and on a column whose field has no doc
/// comment, because a leaf's `--help` describes each column with it.
///
/// ```
/// use crumbkit_core::Row;
/// use schemars::JsonSchema;
/// use serde::Serialize;
///
/// #[derive(Serialize, JsonSchema)]
/// struct Book {
///     id: u32,
///     title: String,
///     pages: u32,
///     isbn: String,
/// }
///
/// impl Row for Book {
///     const COLUMNS: &'static [&'static str] = &["id", "title"];
///     const LONG_COLUMNS: &'static [&'static str] = &["pages"];
/// }
/// ```
///
/// Here `isbn` is in neither list, so it appears only with `--json --full`.
pub trait Row: Serialize + JsonSchema {
    /// The columns that the compact output shows, in display order.
    const COLUMNS: &'static [&'static str];

    /// The columns that `--long` adds after `COLUMNS`, in display order. A
    /// field in neither list appears only with `--json --full`.
    const LONG_COLUMNS: &'static [&'static str] = &[];
}

/// A result that prints as a table: one [`Row`], a `Vec` of rows, a
/// [`Listing`](crate::Listing) or a [`Batch`](crate::Batch).
///
/// crumbkit implements this trait, and a tool cannot implement it, so crumbkit
/// can add result shapes later without breaking a tool.
pub trait Rows: Serialize + sealed::Sealed {
    /// The type of each row.
    type Row: Row;

    /// The kind of result, which decides the fields of its JSON document. A
    /// single row's `data` is an object, and every other result's is an
    /// array.
    const SHAPE: Shape;

    /// The type of the summary that prints above the table. Only a
    /// `Listing` can have a summary, and every other result has
    /// [`NoSummary`](crate::NoSummary). The summary's JSON Schema is the
    /// schema of the document's `summary` field.
    type Summary: Serialize + std::fmt::Display + JsonSchema;

    /// Returns the rows in order. A single row is a slice of one.
    fn rows(&self) -> &[Self::Row];

    /// Returns the number of rows that matched when the result is one page
    /// of them, and `None` otherwise. Only a `Listing` returns a total.
    fn total(&self) -> Option<usize> {
        None
    }

    /// Returns the phrase that names the rows in the text footer, such as
    /// `books`. Only a `Listing` returns one.
    fn description(&self) -> Option<&str> {
        None
    }

    /// Returns the summary that the text output prints above the table and
    /// the JSON document holds under `summary`. Only a `Listing` returns one.
    fn summary(&self) -> Option<&Self::Summary> {
        None
    }

    /// Returns the line that the text output prints after the table and any
    /// footer. Only a `Listing` returns one.
    fn note(&self) -> Option<&str> {
        None
    }

    /// Returns the command that the text output names as the next step, and
    /// the phrase that follows it. Only a `Listing` returns one.
    fn next_step(&self) -> Option<(&Call, &str)> {
        None
    }

    /// Returns the failures of the ids that did not work when the result is
    /// a batch, and `None` otherwise. Only a `Batch` returns them, and its
    /// list is empty when every id worked.
    ///
    /// A `Batch` builds the list on each call, because the trait cannot name
    /// the batch's failure type. `finish` calls it once.
    fn failures(&self) -> Option<Vec<&dyn Failure>> {
        None
    }
}

/// The kind of a [`Rows`] result, which decides the fields of its JSON
/// document beside `schema`.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// One row, whose `data` is an object.
    Row,
    /// A `Vec` of rows, whose `data` is an array.
    List,
    /// A [`Listing`](crate::Listing), whose `data` is an array beside
    /// `total` and, when it has one, `summary`.
    Listing,
    /// A [`Batch`](crate::Batch), whose `data` is an array beside
    /// `failures`.
    Batch,
}

impl<R: Row> Rows for R {
    type Row = R;
    const SHAPE: Shape = Shape::Row;
    type Summary = crate::NoSummary;

    fn rows(&self) -> &[R] {
        std::slice::from_ref(self)
    }
}

impl<R: Row> Rows for Vec<R> {
    type Row = R;
    const SHAPE: Shape = Shape::List;
    type Summary = crate::NoSummary;

    fn rows(&self) -> &[R] {
        self
    }
}

/// Holds the supertrait that keeps tools from implementing [`Rows`].
mod sealed {
    /// A type that crumbkit implements [`Rows`](super::Rows) for. The trait is
    /// public so it can bound `Rows`, and it sits in a private module so no
    /// tool can name it.
    pub trait Sealed {}

    impl<R: super::Row> Sealed for R {}

    impl<R: super::Row> Sealed for Vec<R> {}

    impl<R: super::Row, S> Sealed for crate::Listing<R, S> {}

    impl<R: super::Row, E: crate::AsFailure> Sealed for crate::Batch<R, E> {}
}
