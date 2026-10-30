//! The trait of a result that `crumbkit::finish` prints, and the kind of
//! that result.

use std::fmt;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::Serialize;

use super::{Row, problems};
use crate::{Call, Failure, Step};

/// A result that `crumbkit::finish` prints: one [`Row`], a `Vec` of rows, a
/// [`Listing`](crate::Listing) or a [`Batch`](crate::Batch), which print as
/// a table, a [`Prose`](crate::Prose), which prints as text, or
/// [`Bytes`](crate::Bytes), which print unchanged.
///
/// crumbkit implements this trait, and a tool cannot implement it, so crumbkit
/// can add result shapes later without breaking a tool. A tool names the
/// trait in a bound, such as `T: Rows`, and an outside tool's generic code
/// reads a result through its three members: [`Rows::rows`],
/// [`Rows::total`] and [`Rows::failures`]. Everything else that
/// `crumbkit::finish` reads sits on the supertrait `Sealed`, in a private
/// module, so a tool cannot call it on a result it holds. A test that reads
/// a concrete result calls the result's own methods, such as
/// [`Listing::rows`](crate::Listing::rows) or
/// [`Batch::failures`](crate::Batch::failures).
pub trait Rows: Serialize + Sealed {
    /// Returns the rows in order. A single row is a slice of one, and a
    /// `Prose` and `Bytes` return none.
    fn rows(&self) -> &[Self::Row];

    /// Returns the number of rows that matched when the result is one page
    /// of them, and `None` otherwise. Only a `Listing` returns a total, and
    /// a listing whose source the tool did not count (`Listing::uncounted`)
    /// or read only in part (`Listing::capped`) returns `None`.
    fn total(&self) -> Option<usize> {
        None
    }

    /// Returns the failures of the ids that did not work when the result is
    /// a batch or the failures that a prose result holds, and `None`
    /// otherwise. Only a `Batch` and a `Prose` return them, and the list is
    /// empty when every id worked.
    ///
    /// The result builds the list on each call, because the trait cannot
    /// name the tool's failure type. `finish` calls it once.
    fn failures(&self) -> Option<Vec<&dyn Failure>> {
        None
    }
}

/// What `crumbkit::finish` reads from a [`Rows`] result. crumbkit implements
/// it for each result shape, and a tool can neither implement nor name it,
/// because it is public only within crumbkit.
//
// RUST-10: an outside tool's own function with a `T: Rows` bound can still
// call these members on `T`, because Rust lets a generic call the members of
// every supertrait of its bounds. Only `Rows::rows`, `Rows::total` and
// `Rows::failures` are supported reads in such a function. These members are
// unsupported there and may change in any release. Rust has no way to keep
// a supertrait's members from a bound short of a private token argument on
// each member, which would add an argument to every call in crumbkit. The
// members are absent from the documentation and from every concrete result.
pub trait Sealed {
    /// The type of each row. A `Prose` and `Bytes` have no rows, and their
    /// row type is [`NoRow`](crate::NoRow).
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

    /// Returns the number of rows that the tool read when the result is a
    /// listing from a capped source, whose source holds more rows beyond
    /// them, and `None` otherwise (`Listing::capped`). The footer gives it
    /// as a lower bound on the rows that matched.
    fn at_least(&self) -> Option<usize> {
        None
    }

    /// Returns whether the source holds rows that the result leaves out:
    /// fewer rows than the total, more rows that the tool did not count, or
    /// rows beyond those the tool read from a capped source. Only a
    /// `Listing` returns true.
    fn more(&self) -> bool {
        false
    }

    /// Returns the phrase that names the rows in the text footer, such as
    /// `books`. Only a `Listing` returns one.
    fn description(&self) -> Option<&str> {
        None
    }

    /// Returns the phrase that names one row in the text footer and cut
    /// line of a listing without a total, such as `book`. Only a `Listing`
    /// returns one.
    fn singular(&self) -> Option<&str> {
        None
    }

    /// Returns the summary that the text output prints above the table and
    /// the JSON document holds under `summary`. Only a `Listing` returns one.
    fn summary(&self) -> Option<&Self::Summary> {
        None
    }

    /// Returns the line that the text output prints on standard error after
    /// the result and any footer, and before any failure. A `Listing`, a
    /// `Batch` and a `Prose` return one when the tool set it.
    fn note(&self) -> Option<&str> {
        None
    }

    /// Returns the line that the text output prints on standard error after
    /// the note under `--long` only. Only a `Listing` returns one, when the
    /// tool set it (ADR-0139).
    fn long_note(&self) -> Option<&str> {
        None
    }

    /// Returns the command that the text output names as the next step, and
    /// the phrase that follows it. Only a `Listing` returns one.
    fn next_step(&self) -> Option<(&Call, &str)> {
        None
    }

    /// Returns the steps that the text output names after the next step, in
    /// order. Only a `Listing` returns any.
    fn steps(&self) -> &[Step] {
        &[]
    }

    /// Returns the most rows that the command's `--limit` takes, which the
    /// text footer names, or `None` when it has no bound. Only a `Listing`
    /// returns one.
    fn limit_bound(&self) -> Option<usize> {
        None
    }

    /// Returns the number of rows that report a problem
    /// ([`Row::is_problem`]), which `crumbkit::finish` reads for the
    /// exit code and the note. Every result counts the rows it holds, and a
    /// listing that [`Listing::from_all`](crate::Listing::from_all) cut to
    /// its limit also counts the rows it cut.
    fn problems(&self) -> usize
    where
        Self: Rows,
    {
        problems::count(self.rows())
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
    /// `None` when the result is not a batch. The recovery uses this call
    /// only when it cannot find the keys among the caller's words, and
    /// otherwise reruns those words with the keys left out.
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
    /// `total` when the tool counted every matching row, `truncated` and, when it has
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

impl<R: Row> Rows for R {
    fn rows(&self) -> &[R] {
        std::slice::from_ref(self)
    }
}

impl<R: Row> Sealed for R {
    type Row = R;
    const SHAPE: Shape = Shape::Row;
    type Summary = crate::NoSummary;
}

impl<R: Row> Rows for Vec<R> {
    fn rows(&self) -> &[R] {
        self
    }
}

impl<R: Row> Sealed for Vec<R> {
    type Row = R;
    const SHAPE: Shape = Shape::List;
    type Summary = crate::NoSummary;
}
