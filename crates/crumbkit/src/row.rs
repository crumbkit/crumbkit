//! The traits that let a result print as a table.

use std::borrow::Cow;
use std::fmt;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Serialize, Serializer};

use crate::{Call, Failure};

mod fields;
pub(crate) mod problems;

pub(crate) use fields::{can_be_empty, properties};

/// A type that prints as one table row.
///
/// The names in `COLUMNS` and `LONG_COLUMNS` are the field names as the type
/// serializes them, after any `#[serde(rename)]`. The fields of a row type
/// are the properties of its JSON Schema. When the type is an enum whose
/// schema lists its variants under `oneOf` or `anyOf`, they also include the
/// properties of each variant, so `--fields` and a leaf's columns can name a
/// field that only some variants have. A name that is not a field is
/// missing in every row, so the text table leaves its column out, and JSON
/// holds `null` for it. crumbkit-test's
/// `check_guide` fails on such a name in the row type of each leaf that the
/// tool's guide gives a document, and on a column whose field has no doc
/// comment, because a leaf's `--help` describes each column with it.
///
/// ```
/// use crumbkit::Row;
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
    ///
    /// The list names at least one field, because the table and the JSON
    /// without `--full` print only these fields and the long ones.
    /// `crumbkit::finish` fails to compile for a single row, a `Vec` or a
    /// `Listing` whose row type has an empty list, and `crumbkit::stream`
    /// fails to compile for such a row type. A batch compiles, because the
    /// text of a headed batch prints each row's bytes and its JSON holds
    /// every field, so a headed row type may name no column. crumbkit-test's
    /// `check_guide` fails a leaf whose batch is not headed and whose row
    /// type has an empty list (ADR-0063). A row type that sets
    /// [`WHOLE`](Row::WHOLE) and declares [`LINE`](Row::LINE) may also name
    /// no column (ADR-0072).
    const COLUMNS: &'static [&'static str];

    /// The columns that `--long` adds after `COLUMNS`, in display order. A
    /// field in neither list appears only with `--json --full`.
    const LONG_COLUMNS: &'static [&'static str] = &[];

    /// The field whose value marks the row as reporting a problem, or `None`
    /// when no field does.
    ///
    /// With the default [`Row::is_problem`], a row reports a problem when
    /// this field serializes to a value other than `null`, `false` or an
    /// empty string, and a missing field reports none. The leaf's help
    /// prints a `Problems:` line that names the field. crumbkit-test's
    /// `check_guide` fails when the field is not one of `COLUMNS`, or when
    /// its type is not a string, a boolean or a value that can be null.
    const PROBLEM: Option<&'static str> = None;

    /// Reports whether the row reports a problem.
    ///
    /// A run whose rows report a problem, and that has no failure, exits
    /// with 1, and its text ends with a note that counts those rows. The
    /// exit reads every row that the result holds. A listing from
    /// `Listing::from_all` also counts the rows that its limit cut. A
    /// listing of one page that the tool fetched counts only that page, so
    /// it puts the rows that report a problem first (OUTPUT-3).
    ///
    /// The default reads the field that [`Row::PROBLEM`] names, and reports
    /// no problem when the type names none. A type whose problem follows
    /// from other fields, such as a state enum, overrides it in place of a
    /// second column that repeats the state as a flag, and leaves `PROBLEM`
    /// as `None` (ADR-0096). Its help then prints no `Problems:` line, and
    /// `check_guide` checks no problem field, so the type shows the state in
    /// one of `COLUMNS` itself.
    ///
    /// ```
    /// use crumbkit::Row;
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
    /// impl Row for Job {
    ///     const COLUMNS: &'static [&'static str] = &["name", "state"];
    ///
    ///     fn is_problem(&self) -> bool {
    ///         matches!(self.state, State::Failed)
    ///     }
    /// }
    /// ```
    fn is_problem(&self) -> bool {
        problems::field_reports(self)
    }

    /// The form of the row's own text line as help shows it, such as
    /// `<POINTER> -> <TARGET>`, or `None` when the row prints as a table
    /// row (ADR-0064).
    ///
    /// When it is set, the default text prints each row's
    /// [`Row::text_line`] in place of the table, without a header. `--long`
    /// and `--fields` still print the table, because they name columns, and
    /// `--json` is unchanged. The leaf's help prints the form on a `Text:`
    /// line. crumbkit-test's `check_guide` fails a form that is not one
    /// non-empty line, and a headed leaf whose row type sets it, because a
    /// headed batch's text prints each row's bytes.
    const LINE: Option<&'static str> = None;

    /// Returns the row's own text line, without a line feed, or `None` when
    /// the row has none. crumbkit reads it only when [`Row::LINE`] is set.
    ///
    /// crumbkit escapes each control character of the line, such as a line
    /// feed as `\n`, and each Unicode format character, such as a
    /// bidirectional override as `\u{202e}`. It ends the line with a line
    /// feed, so each row stays one line and shows its text in the order it
    /// was written. A row that returns `None` prints as one line of compact
    /// JSON with every field.
    fn text_line(&self) -> Option<String> {
        None
    }

    /// Whether `--json` prints every field of each row as the type
    /// serializes it, without `--full`. The default is `false`, which
    /// prints only the columns without `--full` (ADR-0065).
    ///
    /// A tool sets it when the readers of its JSON expect each row whole,
    /// such as a field that the type leaves out when it holds nothing. The
    /// text still prints the table of `COLUMNS`, `--long` still adds
    /// `LONG_COLUMNS` to the table, and `--fields` still selects fields,
    /// printing `null` for a field that a row leaves out. crumbkit refuses
    /// `--full`, because the JSON already holds every field.
    ///
    /// A row type that sets it may name no columns, as for a row that
    /// serializes as an array, only when it also declares [`Row::LINE`]
    /// (ADR-0072). Its text then prints each row's own line with no header,
    /// or one line of compact JSON for a row whose [`Row::text_line`]
    /// returns `None`, and crumbkit refuses `--fields` and `--long`.
    /// `crumbkit::finish` and `crumbkit::stream` fail to compile for such a
    /// row type without a `LINE`, and crumbkit-test's `check_guide` fails a
    /// batch of it. `check_guide` also fails a row type without columns
    /// whose schema has properties, which could be columns.
    ///
    /// crumbkit-test's `check_document_ids` fails a leaf whose row type sets
    /// `WHOLE` unless an opt-out from `documents/rows-whole` with a reason
    /// covers the leaf.
    const WHOLE: bool = false;
}

/// A row type that holds bytes, such as a file's contents, so a batch of
/// such rows can print in text the way `head` prints several files.
///
/// [`Document::headed`](crate::Document::headed) marks a batch's document
/// for this text form. `crumbkit::finish` then prints each row's bytes
/// unchanged, after a line `==> KEY <==` that names the row's key when the
/// caller gave more than one key, in place of a table. The row usually
/// holds its bytes in a field marked `#[serde(flatten)]`, so its JSON has
/// `text` or `base64` at the top level ([`Bytes`](crate::Bytes)).
///
/// ```
/// use crumbkit::{Bytes, Headed, Row};
/// use schemars::JsonSchema;
/// use serde::Serialize;
///
/// /// A file and its contents.
/// #[derive(Serialize, JsonSchema)]
/// struct File {
///     /// The file's path.
///     path: String,
///     #[serde(flatten)]
///     content: Bytes,
/// }
///
/// impl Row for File {
///     const COLUMNS: &'static [&'static str] = &["path"];
/// }
///
/// impl Headed for File {
///     fn bytes(&self) -> Option<&Bytes> {
///         Some(&self.content)
///     }
/// }
/// ```
pub trait Headed: Row {
    /// Returns the bytes that print under the row's header in text, or
    /// `None` when the row holds no bytes, such as a row that holds a parsed
    /// record in their place. A row without bytes prints under its header
    /// as one line of compact JSON with every field.
    fn bytes(&self) -> Option<&crate::Bytes>;
}

/// A result that `crumbkit::finish` prints: one [`Row`], a `Vec` of rows, a
/// [`Listing`](crate::Listing) or a [`Batch`](crate::Batch), which print as
/// a table, a [`Prose`](crate::Prose), which prints as text, or
/// [`Bytes`](crate::Bytes), which print unchanged.
///
/// crumbkit implements this trait, and a tool cannot implement it, so crumbkit
/// can add result shapes later without breaking a tool. A tool names the
/// trait only in a bound, such as `T: Rows`, and only crumbkit calls its
/// members, so they are hidden from the documentation.
pub trait Rows: Serialize + sealed::Sealed {
    /// The type of each row. A `Prose` and `Bytes` have no rows, and their
    /// row type is [`NoRow`].
    #[doc(hidden)]
    type Row: Row;

    /// The kind of result, which decides the fields of its JSON document. A
    /// single row's `data` is an object, a prose result's is its value, a
    /// bytes result's is its text or base64 object, and every other
    /// result's is an array.
    #[doc(hidden)]
    const SHAPE: Shape;

    /// The type of the summary that prints above the table. Only a
    /// `Listing` can have a summary, and every other result has
    /// [`NoSummary`](crate::NoSummary). The summary's JSON Schema is the
    /// schema of the document's `summary` field.
    #[doc(hidden)]
    type Summary: Serialize + fmt::Display + JsonSchema;

    /// Returns the rows in order. A single row is a slice of one.
    #[doc(hidden)]
    fn rows(&self) -> &[Self::Row];

    /// Returns the number of rows that matched when the result is one page
    /// of them, and `None` otherwise. Only a `Listing` returns a total, and
    /// a listing whose source the tool did not count returns `None`
    /// (`Listing::uncounted`).
    #[doc(hidden)]
    fn total(&self) -> Option<usize> {
        None
    }

    /// Returns whether the source holds rows that the result leaves out:
    /// fewer rows than the total, or more rows that the tool did not count.
    /// Only a `Listing` returns true.
    #[doc(hidden)]
    fn more(&self) -> bool {
        false
    }

    /// Returns the phrase that names the rows in the text footer, such as
    /// `books`. Only a `Listing` returns one.
    #[doc(hidden)]
    fn description(&self) -> Option<&str> {
        None
    }

    /// Returns the phrase that names one row in the text footer and cut
    /// line of a listing without a total, such as `book`. Only a `Listing`
    /// returns one.
    #[doc(hidden)]
    fn singular(&self) -> Option<&str> {
        None
    }

    /// Returns the summary that the text output prints above the table and
    /// the JSON document holds under `summary`. Only a `Listing` returns one.
    #[doc(hidden)]
    fn summary(&self) -> Option<&Self::Summary> {
        None
    }

    /// Returns the line that the text output prints on standard error after
    /// the result and any footer, and before any failure. A `Listing`, a
    /// `Batch` and a `Prose` return one when the tool set it.
    #[doc(hidden)]
    fn note(&self) -> Option<&str> {
        None
    }

    /// Returns the command that the text output names as the next step, and
    /// the phrase that follows it. Only a `Listing` returns one.
    #[doc(hidden)]
    fn next_step(&self) -> Option<(&Call, &str)> {
        None
    }

    /// Returns the most rows that the command's `--limit` takes, which the
    /// text footer names, or `None` when it has no bound. Only a `Listing`
    /// returns one.
    #[doc(hidden)]
    fn limit_bound(&self) -> Option<usize> {
        None
    }

    /// Returns the number of rows that report a problem
    /// ([`Row::is_problem`]), which `crumbkit::finish` reads for the
    /// exit code and the note. Every result counts the rows it holds, and a
    /// listing that [`Listing::from_all`](crate::Listing::from_all) cut to
    /// its limit also counts the rows it cut.
    #[doc(hidden)]
    fn problems(&self) -> usize {
        problems::count(self.rows())
    }

    /// Returns the failures of the ids that did not work when the result is
    /// a batch, and `None` otherwise. Only a `Batch` returns them, and its
    /// list is empty when every id worked.
    ///
    /// A `Batch` builds the list on each call, because the trait cannot name
    /// the batch's failure type. `finish` calls it once.
    #[doc(hidden)]
    fn failures(&self) -> Option<Vec<&dyn Failure>> {
        None
    }

    /// Returns the keys that a batch leaves out when it shows only its first
    /// `shown` rows, in the order the caller gave them: the keys of the rows
    /// after the first `shown`, then the keys the handler never fetched. A
    /// row whose result was pushed after the last key has no key and is
    /// `None`. Every other result leaves out nothing.
    #[doc(hidden)]
    fn left_out(&self, _shown: usize) -> Vec<Option<&str>> {
        Vec::new()
    }

    /// Returns the position of each failure that [`Rows::failures`]
    /// returns among a batch's pushed results, counted from 0, which for a
    /// failure with a key is the position of its key. Every result that is
    /// not a batch returns no position.
    #[doc(hidden)]
    fn failure_indexes(&self) -> &[usize] {
        &[]
    }

    /// Returns the key at `index` among a batch's keys, or `None` past the
    /// last key and for every result that is not a batch.
    #[doc(hidden)]
    fn key(&self, _index: usize) -> Option<&str> {
        None
    }

    /// Returns the position of each row's result among a batch's pushed
    /// results, which for a row with a key is the position of its key.
    /// Every result that is not a batch returns no position.
    #[doc(hidden)]
    fn row_indexes(&self) -> &[usize] {
        &[]
    }

    /// Returns the position of the first answer that a batch leaves out
    /// when it shows only its first `shown` rows, or `None` when it leaves
    /// out nothing. Every answer from that position on that is not a
    /// failure is left out. Every other result returns `None`.
    #[doc(hidden)]
    fn left_out_from(&self, _shown: usize) -> Option<usize> {
        None
    }

    /// Returns the call that fetches the keys a batch leaves out, with every
    /// value of the over-budget failure's `left_out` field in one run, and
    /// `None` when the result is not a batch. The recovery uses this call
    /// only when it cannot find the keys among the caller's words, and
    /// otherwise reruns those words with the keys left out.
    #[doc(hidden)]
    fn fetch(&self) -> Option<Call> {
        None
    }

    /// Returns the text that the text output prints in place of a table
    /// when the result is a `Prose`, and `None` otherwise. Only a `Prose`
    /// returns one.
    #[doc(hidden)]
    fn prose(&self) -> Option<&dyn fmt::Display> {
        None
    }

    /// Returns the JSON Schema of the document's `data` when the result is a
    /// `Prose`, which is the schema of its value, and `None` otherwise. The
    /// schema comes from `generator`, so a definition it names sits under the
    /// document's `$defs`. Only a `Prose` returns one.
    #[doc(hidden)]
    fn prose_schema(_generator: &mut SchemaGenerator) -> Option<Schema> {
        None
    }

    /// Returns the bytes that the text output prints unchanged when the
    /// result is [`Bytes`](crate::Bytes), and `None` otherwise.
    #[doc(hidden)]
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
