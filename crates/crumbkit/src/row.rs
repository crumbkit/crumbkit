//! The traits that let a result print as a table.

use schemars::JsonSchema;
use serde::Serialize;

mod children;
mod fields;
mod line;
mod no_row;
mod problem;
pub(crate) mod problems;
mod rows;

pub use children::Children;
pub use line::Line;
pub use no_row::NoRow;
pub use problem::Problem;
pub(crate) use rows::Sealed;
pub use rows::{Rows, Shape};

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
/// `check_help` fails on such a name in the row type of each leaf that the
/// tool record gives a document, and on a column whose field has no doc
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
///
/// With the `derive` feature, a struct with named fields derives the impl
/// with `#[derive(Row)]` and marks each column with `#[row(column)]` or
/// `#[row(long)]`. The derive writes each name as serde serializes the
/// field, so a column always names a field. It writes `COLUMNS`,
/// `LONG_COLUMNS`, `UNCUT_COLUMNS` and `PROBLEM`, and crumbkit-derive
/// documents its keys. An enum and a row that sets any other item write the
/// impl by hand.
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
    /// `check_help` fails a leaf whose batch is not headed and whose row
    /// type has an empty list (ADR-0063). A row type that sets
    /// [`WHOLE`](Row::WHOLE) and declares [`LINE`](Row::LINE) may also name
    /// no column (ADR-0072).
    const COLUMNS: &'static [&'static str];

    /// The columns that `--long` adds after `COLUMNS`, in display order. A
    /// field in neither list appears only with `--json --full`.
    const LONG_COLUMNS: &'static [&'static str] = &[];

    /// The columns whose cells a fitted text table never cuts, such as an
    /// identifier that a person copies into the next command. The default
    /// names none (ADR-0121).
    ///
    /// When a text table is wider than the width that the sink gives it,
    /// crumbkit narrows its widest columns and cuts their long cells.
    /// `finish` gives its sink the width of the terminal that standard
    /// output is, and a tool gives its own sink a width with
    /// [`Sink::fitted`](crate::Sink::fitted) (ADR-0134). crumbkit skips the
    /// columns named here, also in a `--fields` table, and narrows the
    /// other columns instead. When these columns and the headers of the
    /// others are wider than that width on their own, the table prints
    /// unfitted, so every cell is whole and a line is wider than the width.
    /// crumbkit-test's `check_help` fails a name here that is in neither
    /// `COLUMNS` nor `LONG_COLUMNS`.
    const UNCUT_COLUMNS: &'static [&'static str] = &[];

    /// Returns the style of the row's cell in `column`, or `None` for a
    /// plain cell. `column` is a field name as the type serializes it, as
    /// in `COLUMNS`. The default styles no cell (ADR-0150).
    ///
    /// A text table writes a cell with its style only when the sink wants
    /// styles for the result: [`finish`](crate::finish) asks for them when
    /// standard output is a terminal that allows colour, and a tool asks
    /// for them on its own sink with [`Sink::styled`](crate::Sink::styled).
    /// The column widths, the fit to a terminal's width and the output
    /// budget count the cell's text alone, so the styled table lines up as
    /// the plain one does, and without its escape codes it is the plain
    /// table. JSON, a row's own text line, child lines, a stream's table and
    /// the header never carry a style. crumbkit calls this only for a table
    /// that a styled sink prints, so a pipe pays nothing for it.
    ///
    /// [`Style`](crate::Style) shows an example.
    fn cell_style(&self, _column: &str) -> Option<crate::Style> {
        None
    }

    /// Which rows report a problem, and the column that shows it, or `None`
    /// when no row does (ADR-0096, ADR-0212). [`Problem`] describes the
    /// exit code, the note and the help that follow.
    const PROBLEM: Option<Problem<Self>> = None;

    /// Reports whether the default text of a listing lists the row, which
    /// is true for every row by default (ADR-0157).
    ///
    /// The default text is the text table, or the rows' own text lines,
    /// without `--json`, `--long` or `--fields`. It lists a row that
    /// reports a problem, as [`Row::PROBLEM`] decides, whatever this
    /// returns, so the note that counts those rows never counts a row that
    /// the text leaves out. It lists each other row for which this returns
    /// true, and none of them when the listing's document is marked with
    /// [`Document::only_problems`](crate::Document::only_problems).
    /// `--long` and `--fields` print every row, and `--json` holds every
    /// row unchanged. A list, a batch, a single row and a stream print
    /// every row.
    ///
    /// A tool whose row type leaves rows out puts the rows that the default
    /// text lists first, as it puts the rows that report a problem first
    /// (OUTPUT-3). A limit and the output budget's cut keep the first rows
    /// of the listing, so with this order neither hides a listed row behind
    /// rows that the text leaves out.
    ///
    /// When the default text leaves out every row, a listing with a summary
    /// line prints the summary line alone, and one without prints nothing
    /// on standard output. The footer counts every row of the page, the
    /// empty note prints only for a listing that holds no rows, and no line
    /// counts the rows that the text left out. The help says nothing about
    /// these rows, because it cannot know which rows the method leaves out,
    /// so the about text of a command whose rows it leaves out says which
    /// views list every row.
    ///
    /// ```
    /// use crumbkit::{Children, Row};
    /// use schemars::JsonSchema;
    /// use serde::Serialize;
    ///
    /// /// An item that a cleanup run removed, or kept with a step for the
    /// /// caller.
    /// #[derive(Serialize, JsonSchema)]
    /// struct Item {
    ///     path: String,
    ///     removed: bool,
    ///     step: Option<String>,
    /// }
    ///
    /// impl Row for Item {
    ///     const COLUMNS: &'static [&'static str] = &["path", "removed", "step"];
    ///
    ///     // A dry run removes nothing, so its text lists every item, and a
    ///     // real run's text lists the items that need the caller's step.
    ///     fn in_default_text(&self) -> bool {
    ///         !self.removed || self.step.is_some()
    ///     }
    /// }
    /// ```
    fn in_default_text(&self) -> bool {
        true
    }

    /// The row's own text line, its form for help and the function that
    /// builds it, or `None` when the row prints as a table row (ADR-0064,
    /// ADR-0213). [`Line`] describes the text and the help that follow.
    const LINE: Option<Line<Self>> = None;

    /// The row's child lines, the field that holds them and the functions
    /// that build them, or `None` when the row has none (ADR-0120,
    /// ADR-0214). [`Children`] describes how the text prints them.
    ///
    /// `--long` prints the lines of [`Children::long_lines`] in their place
    /// when the type sets it, and `--fields` prints none: a `--fields` that
    /// names the field prints it as a cell of compact JSON like any other
    /// field. The JSON without `--full` holds the field after the columns,
    /// so `--json` carries the children with each row. The leaf's help
    /// describes the child lines on a `Children:` line, with
    /// [`Children::help`] or a sentence that names the field. crumbkit-test's `check_help` fails a field that is not a
    /// field of the row type or has no doc comment, a field that is also a
    /// column, and a headed leaf whose row type sets it, because a headed
    /// batch's text prints each row's bytes.
    const CHILDREN: Option<Children<Self>> = None;

    /// Whether `--json` prints every field of each row as the type
    /// serializes it, without `--full`. The default is `false`, which
    /// prints only the columns without `--full` (ADR-0065).
    ///
    /// A tool sets it when the readers of its JSON expect each row whole,
    /// such as a field that the type leaves out when it holds nothing. The
    /// text still prints the table of `COLUMNS`, `--long` still adds
    /// `LONG_COLUMNS` to the table, and `--fields` still selects fields,
    /// printing `null` for a field that a row leaves out. `--long` and
    /// `--full` with `--json` print the document that `--json` alone
    /// prints, with a note on standard error that the flag adds nothing,
    /// because the JSON already holds every field (ADR-0149).
    ///
    /// A row type that sets it may name no columns, as for a row that
    /// serializes as an array, only when it also declares [`Row::LINE`]
    /// (ADR-0072). Its text then prints each row's own line with no header,
    /// or one line of compact JSON for a row whose [`Line::text`] returns
    /// `None`, and crumbkit refuses `--fields`, and `--long` without
    /// `--json`.
    /// `crumbkit::finish` and `crumbkit::stream` fail to compile for such a
    /// row type without a `LINE`, and crumbkit-test's `check_help` fails a
    /// batch of it. `check_help` also fails a row type without columns
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
