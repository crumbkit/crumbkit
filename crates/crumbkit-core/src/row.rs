//! The traits that let a result print as a table.

use std::borrow::Cow;
use std::fmt;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Serialize, Serializer};

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

    /// The field whose value marks the row as reporting a problem, or `None`
    /// when the row type never reports one.
    ///
    /// A row reports a problem when this field serializes to a value other
    /// than `null`, `false` or an empty string, and a missing field reports
    /// none. A run whose rows report a problem, and that has no failure,
    /// exits with 1, and its text ends with a note that counts those rows.
    /// The exit reads only the rows that the result holds, so a listing puts
    /// the rows that report a problem first (OUTPUT-3). crumbkit-test's
    /// `check_guide` fails when the field is not one of `COLUMNS`, or when
    /// its type is not a string, a boolean or a value that can be null.
    const PROBLEM: Option<&'static str> = None;
}

/// A result that `crumbkit::finish` prints: one [`Row`], a `Vec` of rows, a
/// [`Listing`](crate::Listing) or a [`Batch`](crate::Batch), which print as
/// a table, a [`Prose`](crate::Prose), which prints as text, or
/// [`Bytes`](crate::Bytes), which print unchanged.
///
/// crumbkit implements this trait, and a tool cannot implement it, so crumbkit
/// can add result shapes later without breaking a tool.
pub trait Rows: Serialize + sealed::Sealed {
    /// The type of each row. A `Prose` and `Bytes` have no rows, and their
    /// row type is [`NoRow`].
    type Row: Row;

    /// The kind of result, which decides the fields of its JSON document. A
    /// single row's `data` is an object, a prose result's is its value, a
    /// bytes result's is its text or base64 object, and every other
    /// result's is an array.
    const SHAPE: Shape;

    /// The type of the summary that prints above the table. Only a
    /// `Listing` can have a summary, and every other result has
    /// [`NoSummary`](crate::NoSummary). The summary's JSON Schema is the
    /// schema of the document's `summary` field.
    type Summary: Serialize + fmt::Display + JsonSchema;

    /// Returns the rows in order. A single row is a slice of one.
    fn rows(&self) -> &[Self::Row];

    /// Returns the number of rows that matched when the result is one page
    /// of them, and `None` otherwise. Only a `Listing` returns a total, and
    /// a listing whose source holds more rows that the tool did not count
    /// returns `None` (`Listing::uncounted`).
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

    /// Returns the most rows that the command's `--limit` takes, which the
    /// text footer names, or `None` when it has no bound. Only a `Listing`
    /// returns one.
    fn limit_bound(&self) -> Option<usize> {
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

    /// Returns the keys that a batch leaves out when it shows only its first
    /// `shown` rows, in the order the caller gave them: the keys of the rows
    /// after the first `shown`, then the keys the handler never fetched. A
    /// row whose result was pushed after the last key has no key and is
    /// `None`. Every other result leaves out nothing.
    fn left_out(&self, _shown: usize) -> Vec<Option<&str>> {
        Vec::new()
    }

    /// Returns the position of each failure that [`Rows::failures`]
    /// returns among a batch's pushed results, counted from 0, which for a
    /// failure with a key is the position of its key. Every result that is
    /// not a batch returns no position.
    fn failure_indexes(&self) -> &[usize] {
        &[]
    }

    /// Returns the key at `index` among a batch's keys, or `None` past the
    /// last key and for every result that is not a batch.
    fn key(&self, _index: usize) -> Option<&str> {
        None
    }

    /// Returns the position of each row's result among a batch's pushed
    /// results, which for a row with a key is the position of its key.
    /// Every result that is not a batch returns no position.
    fn row_indexes(&self) -> &[usize] {
        &[]
    }

    /// Returns the position of the first answer that a batch leaves out
    /// when it shows only its first `shown` rows, or `None` when it leaves
    /// out nothing. Every answer from that position on that is not a
    /// failure is left out. Every other result returns `None`.
    fn left_out_from(&self, _shown: usize) -> Option<usize> {
        None
    }

    /// Returns the call that fetches the keys a batch leaves out, with every
    /// value of the over-budget failure's `left_out` field in one run, and
    /// `None` when the result is not a batch.
    fn fetch(&self) -> Option<Call> {
        None
    }

    /// Returns the text that the text output prints in place of a table
    /// when the result is a `Prose`, and `None` otherwise. Only a `Prose`
    /// returns one.
    fn prose(&self) -> Option<&dyn fmt::Display> {
        None
    }

    /// Returns the JSON Schema of the document's `data` when the result is a
    /// `Prose`, which is the schema of its value, and `None` otherwise. The
    /// schema comes from `generator`, so a definition it names sits under the
    /// document's `$defs`. Only a `Prose` returns one.
    fn prose_schema(_generator: &mut SchemaGenerator) -> Option<Schema> {
        None
    }

    /// Returns the bytes that the text output prints unchanged when the
    /// result is [`Bytes`](crate::Bytes), and `None` otherwise.
    fn bytes(&self) -> Option<&[u8]> {
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
    /// `total` when the tool counted the rows, `truncated` and, when it has
    /// one, `summary`.
    Listing,
    /// A [`Batch`](crate::Batch), whose `data` is an array beside
    /// `failures`.
    Batch,
    /// A [`Prose`](crate::Prose), whose `data` is its value beside
    /// `failures`.
    Prose,
    /// [`Bytes`](crate::Bytes), whose `data` is an object that holds the
    /// bytes as `text` or `base64`.
    Bytes,
}

/// The row type of a result without rows, which is a
/// [`Prose`](crate::Prose). It has no values and no columns, so a result of
/// this row type never holds a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoRow {}

impl Row for NoRow {
    const COLUMNS: &'static [&'static str] = &[];
}

impl Serialize for NoRow {
    fn serialize<S: Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        match *self {}
    }
}

/// Describes `NoRow` as the schema `false`, which no value matches, because
/// a result of this row type never holds a row. The schema is inline, so a
/// generator returns `false` itself and adds no definition.
impl JsonSchema for NoRow {
    fn inline_schema() -> bool {
        true
    }

    fn schema_name() -> Cow<'static, str> {
        "NoRow".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        Schema::from(false)
    }
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

    impl<T, E: crate::AsFailure> Sealed for crate::Prose<T, E> {}

    impl Sealed for crate::Bytes {}
}
