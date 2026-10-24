//! The traits that let a result print as a table.

use schemars::JsonSchema;
use serde::Serialize;

mod children;
mod fields;
mod no_row;
pub(crate) mod problems;
mod rows;

pub use no_row::NoRow;
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
    /// crumbkit-test's `check_guide` fails a name here that is in neither
    /// `COLUMNS` nor `LONG_COLUMNS`.
    const UNCUT_COLUMNS: &'static [&'static str] = &[];

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

    /// The field that holds the row's children, as the type serializes it,
    /// after any `#[serde(rename)]`, or `None` when the row has none
    /// (ADR-0120).
    ///
    /// When it is set, the text prints each row's [`Row::child_lines`]
    /// under the row's table line, or under its own text line when the type
    /// sets [`Row::LINE`], each indented by [`Row::CHILD_INDENT`] spaces
    /// (ADR-0135). `--long` prints [`Row::long_child_lines`] in their
    /// place, and `--fields` prints none: a `--fields` that names the
    /// field prints it as a cell of compact JSON like any other field. The
    /// JSON without `--full` holds the field after the columns, so `--json`
    /// carries the children with each row. The leaf's help describes the
    /// child lines on a `Children:` line, with [`Row::CHILDREN_HELP`] or a
    /// sentence that names the field. crumbkit-test's `check_guide` fails a
    /// field that is not a field of the row type or has no doc comment, a
    /// field that is also a column, and a headed leaf whose row type sets
    /// it, because a headed batch's text prints each row's bytes.
    const CHILDREN: Option<&'static str> = None;

    /// The number of spaces before each of the row's
    /// [`Row::child_lines`] in text, 2 by default (ADR-0135).
    ///
    /// A type that sets 0 prints each child line at column 1, as a table
    /// line starts, so a reader tells the two apart only by the line's
    /// content, which the doc comment of the [`Row::CHILDREN`] field should
    /// describe. The type is `u8`, so the compiler rejects an indent above
    /// 255. The output budget counts the spaces. Help's own `Children:`
    /// sentence calls the lines indented only when the indent is above 0
    /// (ADR-0137).
    const CHILD_INDENT: u8 = 2;

    /// The description of the child lines that help prints after
    /// `Children: `, ending with a full stop, or `None` for crumbkit's
    /// sentence (ADR-0137).
    ///
    /// crumbkit's sentence is `one indented line under each row for each of
    /// its <FIELD>.`, without `indented` when [`Row::CHILD_INDENT`] is 0. A
    /// type whose [`Row::child_lines`] are not one line per item sets its
    /// own text, such as `one line for each application, or one folded
    /// line when every application is stopped.`. `-h`, `--help`, the
    /// `children_help` field of `commands list` and crumbkit-markdown's
    /// reference print the same text. crumbkit-test's `check_guide` fails
    /// a text that is empty, holds a control character or does not end
    /// with a full stop, and a text on a row type without
    /// [`Row::CHILDREN`], whose help prints no `Children:` line.
    const CHILDREN_HELP: Option<&'static str> = None;

    /// Returns one text line for each of the row's children, without
    /// indentation or a line feed. crumbkit reads it only when
    /// [`Row::CHILDREN`] is set.
    ///
    /// crumbkit prints each line under the row, after
    /// [`Row::CHILD_INDENT`] spaces and followed by a line feed. It escapes each control character and each
    /// Unicode format character of the line as a table cell escapes them,
    /// so each child stays on one line. A row that returns no lines prints
    /// nothing under it. The child lines do not change the widths of the
    /// table's columns, and the output budget counts them.
    ///
    /// The default serializes the row as its text cells are built and
    /// returns the items of the [`Row::CHILDREN`] field when it is an array
    /// of strings, and no lines otherwise. A field whose items are
    /// [`Shown`](crate::Shown) values, or strings, therefore prints one child
    /// line for each item, its `Display`, with no override:
    ///
    /// ```
    /// use std::fmt;
    ///
    /// use crumbkit::{Row, Shown};
    /// use schemars::JsonSchema;
    /// use serde::Serialize;
    ///
    /// /// One part of a service, which prints as `+ database` when it answers.
    /// struct Part {
    ///     name: &'static str,
    ///     up: bool,
    /// }
    ///
    /// impl fmt::Display for Part {
    ///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    ///         let mark = if self.up { '+' } else { '-' };
    ///         write!(f, "{mark} {}", self.name)
    ///     }
    /// }
    ///
    /// impl Shown for Part {}
    /// crumbkit::shown!(Part);
    ///
    /// /// A service and the parts it runs on.
    /// #[derive(Serialize, JsonSchema)]
    /// struct Service {
    ///     /// The service's name.
    ///     name: String,
    ///     /// The parts that the service runs on.
    ///     parts: Vec<Part>,
    /// }
    ///
    /// impl Row for Service {
    ///     const COLUMNS: &'static [&'static str] = &["name"];
    ///     const CHILDREN: Option<&'static str> = Some("parts");
    /// }
    ///
    /// let service = Service {
    ///     name: "catalogue".to_owned(),
    ///     parts: vec![Part { name: "database", up: true }],
    /// };
    /// assert_eq!(service.child_lines(), ["+ database"]);
    /// ```
    ///
    /// A type whose children print otherwise overrides it:
    ///
    /// ```
    /// use crumbkit::Row;
    /// use schemars::JsonSchema;
    /// use serde::Serialize;
    ///
    /// /// One part of a service.
    /// #[derive(Serialize, JsonSchema)]
    /// struct Component {
    ///     /// The part's name.
    ///     name: String,
    ///     /// Whether the part answers.
    ///     up: bool,
    /// }
    ///
    /// /// A service and the parts it runs on.
    /// #[derive(Serialize, JsonSchema)]
    /// struct Service {
    ///     /// The service's name.
    ///     name: String,
    ///     /// The parts that the service runs on.
    ///     components: Vec<Component>,
    /// }
    ///
    /// impl Row for Service {
    ///     const COLUMNS: &'static [&'static str] = &["name"];
    ///     const CHILDREN: Option<&'static str> = Some("components");
    ///
    ///     fn child_lines(&self) -> Vec<String> {
    ///         let mark = |up: bool| if up { '+' } else { '-' };
    ///         self.components
    ///             .iter()
    ///             .map(|part| format!("{} {}", mark(part.up), part.name))
    ///             .collect()
    ///     }
    /// }
    /// ```
    fn child_lines(&self) -> Vec<String> {
        children::shown_lines(self, Self::CHILDREN)
    }

    /// Returns the child lines that the text prints under the row with
    /// `--long`, in place of [`Row::child_lines`], which the default
    /// returns. crumbkit reads it only when [`Row::CHILDREN`] is set, and
    /// prints each line as it prints a child line (ADR-0138).
    fn long_child_lines(&self) -> Vec<String> {
        self.child_lines()
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
