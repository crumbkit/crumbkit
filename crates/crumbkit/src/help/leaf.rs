//! Leaves, which give a command without subcommands examples in its help
//! and let its `--help` describe each output column.

use clap::builder::StyledStr;
use clap::builder::styling::Style;
use schemars::{JsonSchema, Schema};

use super::view::{self, Kind, Printed, Unoffered, ViewFlag};
use super::{Effects, write_line};
use crate::pick::Choices;
use crate::{Call, Document, Row, Rows, Shape, Stream, row};

mod children;
mod codes;
mod column;
mod example;
mod passes;
mod refusal;
#[cfg(test)]
mod tests;
mod text;

pub use children::children_line;
pub use column::Column;
pub use example::Example;
pub(crate) use example::examples;
pub use refusal::RefusalContext;
pub use text::text_line;

/// The line that the `-h` and `--help` of a leaf set with
/// [`Leaf::stream`] print after the effects line, in help's wording
/// (ADR-0041). Another rendering of help, such as a Markdown reference,
/// prints this line so that it describes a stream as help does.
pub const STREAM_LINE: &str =
    "Output: one row per line as each arrives, and --json prints one JSON document per line.";

/// The line that the `-h` and `--help` of a leaf whose document holds
/// [`Bytes`](crate::Bytes) print after the effects line, in help's wording.
pub const BYTES_LINE: &str =
    "Output: the bytes unchanged, and --json prints them as text or base64.";

/// The line that the `-h` and `--help` of a leaf whose document is headed
/// print after the effects line, in help's wording (ADR-0062).
pub const HEADED_LINE: &str = "Output: each answer's bytes unchanged, under a ==> KEY <== header when there are several keys, and --json prints the bytes as text or base64.";

/// A command without subcommands whose help shows examples, and whose
/// `--help` describes the columns of the result it prints.
///
/// A tool lists its leaves in its [`Guide`](super::Guide):
///
/// ```
/// # #[derive(serde::Serialize, schemars::JsonSchema)]
/// # struct Book { id: u32 }
/// # impl crumbkit::Row for Book { const COLUMNS: &'static [&'static str] = &["id"]; }
/// use crumbkit::Document;
/// use crumbkit::help::{Example, Guide, Leaf};
///
/// const BOOKS_LIST: Document<Vec<Book>> =
///     Document::new("shelf", "books-list", 1, "The books in the catalogue.");
///
/// const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "list"])
///     .examples(&[
///         Example::new(&[], "Print the first page of books"),
///         Example::new(&["--all"], "Print every book"),
///     ])
///     .document(BOOKS_LIST)]);
///
/// assert_eq!(GUIDE.get_leaves()[0].path(), ["books", "list"]);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Leaf {
    /// The names of the subcommands from the root to the command.
    path: &'static [&'static str],
    /// The examples, in the order the help page lists them.
    examples: &'static [Example],
    /// The document that the command prints and the columns of its rows,
    /// or `None` when the leaf has no document.
    columns: Option<Columns>,
    /// The command's effects, or `None` when the tool declares none.
    effects: Option<Effects>,
    /// The sources of the values of the command's arguments, which a picker
    /// offers when an argument is missing.
    choices: &'static [Choices],
    /// The program whose exit code the command passes on, or `None` when
    /// the command exits as the guide's exits declare.
    passes_on_exit: Option<&'static str>,
    /// The function that builds the calls that a refusal of a command line
    /// that names the command offers as its recovery and alternatives.
    refusal_alternatives: Option<fn(&RefusalContext<'_>) -> Vec<Call>>,
    /// The function that returns the hint that a refusal of a command line
    /// that names the command offers as its recovery.
    refusal_hint: Option<fn(&RefusalContext<'_>) -> Option<String>>,
    /// The tool's own codes for crumbkit's failures in the command's runs
    /// (ADR-0060).
    codes: &'static [super::Rename],
}

impl Leaf {
    /// Returns the leaf of the command at `path`, the real names of its
    /// subcommands from the root, as [`Group::new`](super::Group::new) names
    /// a command. The leaf has no examples and no document.
    #[must_use]
    pub const fn new(path: &'static [&'static str]) -> Self {
        Self {
            path,
            examples: &[],
            columns: None,
            effects: None,
            choices: &[],
            passes_on_exit: None,
            refusal_alternatives: None,
            refusal_hint: None,
            codes: &[],
        }
    }

    /// Returns the names of the subcommands from the root to the command.
    #[must_use]
    pub const fn path(&self) -> &'static [&'static str] {
        self.path
    }

    /// Sets the examples that the command's help lists, in order.
    #[must_use]
    pub const fn examples(mut self, examples: &'static [Example]) -> Self {
        self.examples = examples;
        self
    }

    /// Returns the examples that [`Leaf::examples`] set. The name has clap's
    /// `get_` prefix, because `examples` is the builder method.
    #[must_use]
    pub const fn get_examples(&self) -> &'static [Example] {
        self.examples
    }

    /// Sets the command's effects. Its `-h` and `--help` then print them on
    /// one line after the examples, such as `Effects: read-only,
    /// closed-world.`, and `commands list` writes them under `effects`.
    ///
    /// A leaf without effects prints no such line, and `commands list`
    /// writes `null`, which a caller reads as a command that may destroy
    /// data. crumbkit-test's `check_effects` fails on a visible leaf without
    /// effects unless the tool opts out with a reason, and its `check_guide`
    /// fails on a leaf whose effects change something and that has no
    /// document, unless the leaf passes on a program's exit code.
    #[must_use]
    pub const fn effects(mut self, effects: Effects) -> Self {
        self.effects = Some(effects);
        self
    }

    /// Returns the effects that [`Leaf::effects`] set, or `None` when the
    /// tool declares none. The name has clap's `get_` prefix, because
    /// `effects` is the builder method.
    #[must_use]
    pub const fn get_effects(&self) -> Option<Effects> {
        self.effects
    }

    /// Sets the sources of the values of the command's arguments, which the
    /// guide's [`Guide::picker`](super::Guide::picker) offers when the
    /// caller leaves a required argument out (ADR-0043). An argument with
    /// possible values offers them and never calls its source.
    /// crumbkit-test's `check_guide` fails on a source whose argument id
    /// names no argument of the command.
    #[must_use]
    pub const fn choices(mut self, choices: &'static [Choices]) -> Self {
        self.choices = choices;
        self
    }

    /// Returns the sources that [`Leaf::choices`] set. The name has clap's
    /// `get_` prefix, because `choices` is the builder method.
    #[must_use]
    pub const fn get_choices(&self) -> &'static [Choices] {
        self.choices
    }

    /// Sets the document that the command prints, the same constant that
    /// the tool passes to [`finish`](crate::finish). The command's `--help`
    /// then lists the columns of the document's row type, each with the doc
    /// comment of its field, and its `-h` points at them. A
    /// [`Prose`](crate::Prose) document has no columns, so its leaf's help
    /// lists none and its `-h` points at none.
    ///
    /// The leaf keeps the document's id and description, the row type's
    /// column lists, and functions that build the JSON Schemas of the row
    /// type and of the whole document, so one guide holds leaves whose rows
    /// have different types. [`schemas`](crate::schemas) serves each
    /// leaf's document from them.
    #[must_use]
    pub const fn document<T: Rows>(mut self, document: Document<T>) -> Self {
        self.columns = Some(Columns {
            document: document.untyped(),
            compact: <T::Row as Row>::COLUMNS,
            long: <T::Row as Row>::LONG_COLUMNS,
            uncut: <T::Row as Row>::UNCUT_COLUMNS,
            problem: <T::Row as Row>::PROBLEM,
            line: <T::Row as Row>::LINE,
            children: <T::Row as Row>::CHILDREN,
            schema: schema::<T::Row>,
            envelope: crate::schemas::envelope::<T>,
            kind: Kind::Document(T::SHAPE),
            whole: <T::Row as Row>::WHOLE,
        });
        self
    }

    /// Returns the leaf with the document and columns of `other`, which
    /// [`Leaf::document`] or [`Leaf::stream`] set, or without a document
    /// when `other` has none. A runtime leaf's document reaches its leaf in
    /// the joined guide this way.
    pub(crate) const fn with_document_of(mut self, other: &Self) -> Self {
        self.columns = other.columns;
        self
    }

    /// Sets the stream document that the command prints, the same constant
    /// that the tool passes to [`stream`](crate::stream). The leaf then
    /// holds the document as [`Leaf::document`] does, and also marks the
    /// command as a stream: its `-h` and `--help` print an `Output:` line
    /// after the effects, `commands list` writes the form `stream`, and
    /// `schemas list` writes the form `stream` for its document (ADR-0041).
    #[must_use]
    pub const fn stream<R: Row>(mut self, document: Document<Stream<R>>) -> Self {
        self.columns = Some(Columns {
            document: document.untyped(),
            compact: R::COLUMNS,
            long: R::LONG_COLUMNS,
            uncut: R::UNCUT_COLUMNS,
            problem: R::PROBLEM,
            line: R::LINE,
            children: R::CHILDREN,
            schema: schema::<R>,
            envelope: crate::schemas::envelope::<R>,
            kind: Kind::Stream,
            whole: R::WHOLE,
        });
        self
    }

    /// Reports whether [`Leaf::stream`] set the leaf's document, so the
    /// command prints a stream of rows, one line for each.
    #[must_use]
    pub const fn is_stream(&self) -> bool {
        matches!(
            self.columns,
            Some(Columns {
                kind: Kind::Stream,
                ..
            })
        )
    }

    /// Returns the `Output:` line that the leaf's `-h` and `--help` print
    /// after the effects line: [`STREAM_LINE`] for a stream,
    /// [`BYTES_LINE`] for a document that holds [`Bytes`](crate::Bytes),
    /// [`HEADED_LINE`] for a headed document, and `None` for any other
    /// leaf.
    pub(crate) const fn output_line(&self) -> Option<&'static str> {
        match self.columns {
            Some(Columns {
                kind: Kind::Stream, ..
            }) => Some(STREAM_LINE),
            Some(Columns {
                kind: Kind::Document(Shape::Bytes),
                ..
            }) => Some(BYTES_LINE),
            Some(columns) if columns.document.is_headed() => Some(HEADED_LINE),
            _ => None,
        }
    }

    /// Reports whether the row type of the leaf's document sets
    /// [`Row::WHOLE`], so `--json` prints every field of each row without
    /// `--full`, or `false` when the leaf has no document (ADR-0065).
    /// crumbkit-test's `check_document_ids` reads it to require an opt-out
    /// for such a leaf.
    #[must_use]
    pub const fn rows_whole(&self) -> bool {
        match self.columns {
            Some(columns) => columns.whole,
            None => false,
        }
    }

    /// Reports whether the leaf's command offers the view flag `flag`, as
    /// its declared document decides (ADR-0016). A prose or bytes document
    /// offers no view flag. Rows offer `--fields`, offer `--long` when the
    /// row type has long columns, and offer `--full` when the document has
    /// crumbkit's envelope. A headed document offers neither `--long` nor
    /// `--full` (ADR-0062). Rows that print whole offer no `--full`, and
    /// offer neither `--fields` nor `--long` when they have no columns
    /// (ADR-0065). A leaf without a document offers every view
    /// flag, because crumbkit cannot tell what its command prints.
    #[must_use]
    pub const fn offers(&self, flag: ViewFlag) -> bool {
        self.unoffered(flag).is_none()
    }

    /// Returns why the leaf's command does not offer `flag`, or `None` when
    /// it offers it, as [`Leaf::offers`] describes.
    pub(crate) const fn unoffered(&self, flag: ViewFlag) -> Option<Unoffered> {
        match self.columns {
            Some(columns) => view::unoffered(&columns.printed(), flag),
            None => None,
        }
    }

    /// Returns the document that [`Leaf::document`] set, without its result
    /// type, or `None` when the leaf has no document. The name has clap's
    /// `get_` prefix, because `document` is the builder method.
    #[must_use]
    pub const fn get_document(&self) -> Option<Document<()>> {
        match self.columns {
            Some(columns) => Some(columns.document),
            None => None,
        }
    }

    /// Returns the field that marks a row of the leaf's document as
    /// reporting a problem, the row type's [`Row::PROBLEM`], or `None` when
    /// the leaf has no document or its row type names no such field.
    #[must_use]
    pub const fn problem(&self) -> Option<&'static str> {
        match self.columns {
            Some(columns) => columns.problem,
            None => None,
        }
    }

    /// Returns the columns whose cells a text table at a terminal never
    /// cuts, the row type's [`Row::UNCUT_COLUMNS`], or no column when the
    /// leaf has no document. crumbkit-test's `check_guide` reads it to fail
    /// a name that is not one of the row type's columns.
    #[must_use]
    pub const fn uncut_columns(&self) -> &'static [&'static str] {
        match self.columns {
            Some(columns) => columns.uncut,
            None => &[],
        }
    }

    /// Returns the `Problems:` line that the leaf's `-h` and `--help` print
    /// after the `Output:` line when its row type names a problem field,
    /// such as `Problems: a value in the problem column marks a row with a
    /// problem, and the command then exits with 1.`, or `None` otherwise.
    pub(crate) fn problems_line(&self) -> Option<String> {
        let field = self.problem()?;
        let exit = crate::exit::problem_rows();
        Some(format!(
            "Problems: a value in the {field} column marks a row with a problem, and the command then exits with {exit}."
        ))
    }

    /// Returns the [`Shape`] of the result that [`Leaf::document`] set, such
    /// as [`Shape::Batch`], or `None` when the leaf has no document or
    /// [`Leaf::stream`] set it. crumbkit-test's `check_effects` reads it to
    /// require that a leaf whose effects are
    /// [`per_item`](crate::help::Effects::per_item) prints a document that
    /// reports each item.
    #[must_use]
    pub const fn shape(&self) -> Option<Shape> {
        match self.columns {
            Some(Columns {
                kind: Kind::Document(shape),
                ..
            }) => Some(shape),
            Some(Columns {
                kind: Kind::Stream, ..
            })
            | None => None,
        }
    }

    /// Returns the JSON Schema of the whole document that the leaf prints,
    /// as [`schemas::document_schema`](crate::schemas::document_schema)
    /// builds it, or `None` when the leaf has no document. It builds the
    /// schema on each call.
    #[must_use]
    pub fn document_schema(&self) -> Option<Schema> {
        self.columns
            .map(|columns| (columns.envelope)(columns.document))
    }

    /// Reports whether the row type of the leaf's document has any field, as
    /// [`Row`] describes the fields of a row type, or `false` when the leaf
    /// has no document. It builds the row type's JSON Schema on each call.
    /// crumbkit-test's `check_guide` reads it to fail on a row type that
    /// sets [`Row::WHOLE`], names no columns and has fields that could be
    /// columns.
    #[must_use]
    pub fn row_has_fields(&self) -> bool {
        self.columns
            .is_some_and(|columns| !row::properties(&(columns.schema)()).is_empty())
    }

    /// Returns the JSON Schema of the row type of the leaf's document, or
    /// `None` when the leaf has no document. It builds the schema on each
    /// call.
    pub(crate) fn row_schema(&self) -> Option<Schema> {
        self.columns.map(|columns| (columns.schema)())
    }

    /// Returns the function that builds the JSON Schema of the whole
    /// document that the leaf prints, as
    /// [`schemas::document_schema`](crate::schemas::document_schema)
    /// builds it, or `None` when the leaf has no document.
    pub(crate) fn envelope(&self) -> Option<fn(Document<()>) -> Schema> {
        self.columns.map(|columns| columns.envelope)
    }

    /// Reports whether the leaf has a document whose row type has columns,
    /// which its `--help` lists. A prose document has none. A headed
    /// document always has some, because its `--help` lists `text` and
    /// `base64` among the JSON fields also when its row type names no
    /// column.
    pub(crate) const fn has_columns(&self) -> bool {
        match self.columns {
            Some(columns) => {
                columns.document.is_headed()
                    || !columns.compact.is_empty()
                    || !columns.long.is_empty()
            }
            None => false,
        }
    }

    /// Returns the columns of the leaf's row type as its `--help` describes
    /// them: each name of [`Row::COLUMNS`] and then each name of
    /// [`Row::LONG_COLUMNS`], in declared order, or no column when the leaf
    /// has no document. A headed document's text has no columns, so its
    /// names are the JSON fields that `--fields` selects: the same names,
    /// none of them long, and then `text` and `base64` (ADR-0062).
    ///
    /// It builds the row type's JSON Schema, so it costs a schema build on
    /// each call. A column's description is the first paragraph of the
    /// `description` of its property in the schema, which schemars takes
    /// from the field's doc comment. The properties are those that
    /// [`Row`] describes as the fields of a row type, so a column of an
    /// enum's variant has a property too. crumbkit's `--help` and
    /// crumbkit-test's `check_guide` both read the columns through this
    /// method.
    //
    // RUST-10: the description comes only from the property itself. A
    // column whose name is not a property, and a field without a doc
    // comment, whose type's doc comment schemars keeps under `$defs`, have
    // no description. crumbkit-test's `check_guide` fails on each such
    // column, so a tool finds them in a test. A reader that follows a
    // property's `$ref` into `$defs` replaces this when a tool needs those
    // descriptions.
    #[must_use]
    pub fn column_descriptions(&self) -> Vec<Column> {
        let Some(columns) = self.columns else {
            return Vec::new();
        };
        let schema = (columns.schema)();
        let properties = row::properties(&schema);
        let headed = columns.document.is_headed();
        let bytes: &[&str] = if headed { &["text", "base64"] } else { &[] };
        let compact = columns.compact.iter().map(|name| (name, false));
        let long = columns.long.iter().map(|name| (name, !headed));
        let bytes = bytes.iter().filter(|name| !columns.compact.contains(name));
        compact
            .chain(long)
            .chain(bytes.map(|name| (name, false)))
            .map(|(&name, long)| Column::read(&properties, name, long))
            .collect()
    }
}

/// The document of a leaf and the columns of its row type, as
/// [`Leaf::document`] records them.
#[derive(Clone, Copy, Debug)]
struct Columns {
    /// The document's id and description.
    document: Document<()>,
    /// The row type's [`Row::COLUMNS`].
    compact: &'static [&'static str],
    /// The row type's [`Row::LONG_COLUMNS`].
    long: &'static [&'static str],
    /// The row type's [`Row::UNCUT_COLUMNS`].
    uncut: &'static [&'static str],
    /// The row type's [`Row::PROBLEM`].
    problem: Option<&'static str>,
    /// The row type's [`Row::LINE`].
    line: Option<&'static str>,
    /// The row type's [`Row::CHILDREN`].
    children: Option<&'static str>,
    /// Builds the row type's JSON Schema, whose properties hold the
    /// columns' descriptions.
    schema: fn() -> Schema,
    /// Builds the JSON Schema of the whole document from its id and
    /// description. A stream's schema describes one line.
    envelope: fn(Document<()>) -> Schema,
    /// The shape of the document's result, or a stream when
    /// [`Leaf::stream`] set the document.
    kind: Kind,
    /// The row type's [`Row::WHOLE`].
    whole: bool,
}

impl Columns {
    /// Returns what the leaf's command prints, as far as the view flags
    /// depend on it.
    const fn printed(&self) -> Printed {
        Printed {
            kind: self.kind,
            columns: self.compact,
            long: self.long,
            envelope: self.document.get_envelope().wraps(),
            headed: self.document.is_headed(),
            whole: self.whole,
        }
    }
}

/// Returns the JSON Schema of `R`, as the document's schema describes it.
fn schema<R: JsonSchema>() -> Schema {
    crate::schemas::settings()
        .into_generator()
        .into_root_schema_for::<R>()
}

/// Returns the sections of the `--help` of `leaf` that describe its
/// columns: `Columns:` with the row type's [`Row::COLUMNS`] and
/// `Columns that --long adds:` with its [`Row::LONG_COLUMNS`], each in
/// declared order, leaving out a section with no names. It returns no
/// section when the leaf has no document. A headed leaf has one section,
/// `JSON fields that --fields selects:`. `header` styles each heading and
/// `literal` each name.
///
/// Each line has the layout of the goal list, and holds the column's name
/// and the description that [`Leaf::column_descriptions`] reads. A column
/// without a description prints its name alone. This builds the schema, so
/// `parse` calls it only for `--help`.
pub(crate) fn columns(leaf: &Leaf, header: Style, literal: Style) -> Vec<String> {
    let columns = leaf.column_descriptions();
    let mut sections = Vec::new();
    let compact = match leaf.columns {
        Some(columns) if columns.document.is_headed() => "JSON fields that --fields selects:",
        _ => "Columns:",
    };
    for (heading, long) in [(compact, false), ("Columns that --long adds:", true)] {
        let section_columns: Vec<&Column> = columns
            .iter()
            .filter(|column| column.long == long)
            .collect();
        let Some(width) = section_columns
            .iter()
            .map(|column| column.name.chars().count())
            .max()
        else {
            continue;
        };
        let mut section = format!("{header}{heading}{header:#}");
        for column in section_columns {
            let about = column.description.clone().map(StyledStr::from);
            write_line(&mut section, column.name, about.as_ref(), width, literal);
        }
        sections.push(section);
    }
    sections
}
