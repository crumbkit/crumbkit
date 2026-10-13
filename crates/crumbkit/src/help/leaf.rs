//! Leaves, which give a command without subcommands examples in its help
//! and let its `--help` describe each output column.

use std::fmt::Write as _;

use clap::builder::StyledStr;
use clap::builder::styling::Style;
use schemars::{JsonSchema, Schema};
use serde_json::Value;

use super::words::quote;
use super::{Effects, write_line};
use crate::pick::Choices;
use crate::{Document, Row, Rows, Shape, Stream, row};

#[cfg(test)]
mod tests;

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
    /// effects unless the tool opts out with a reason.
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
            problem: <T::Row as Row>::PROBLEM,
            schema: schema::<T::Row>,
            envelope: crate::schemas::envelope::<T>,
            shape: Some(T::SHAPE),
            stream: false,
        });
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
            problem: R::PROBLEM,
            schema: schema::<R>,
            envelope: crate::schemas::envelope::<R>,
            shape: None,
            stream: true,
        });
        self
    }

    /// Reports whether [`Leaf::stream`] set the leaf's document, so the
    /// command prints a stream of rows, one line for each.
    #[must_use]
    pub const fn is_stream(&self) -> bool {
        match self.columns {
            Some(columns) => columns.stream,
            None => false,
        }
    }

    /// Returns the `Output:` line that the leaf's `-h` and `--help` print
    /// after the effects line: [`STREAM_LINE`] for a stream,
    /// [`BYTES_LINE`] for a document that holds [`Bytes`](crate::Bytes),
    /// and `None` for any other leaf.
    #[must_use]
    pub const fn output_line(&self) -> Option<&'static str> {
        match self.columns {
            Some(columns) if columns.stream => Some(STREAM_LINE),
            Some(Columns {
                shape: Some(Shape::Bytes),
                ..
            }) => Some(BYTES_LINE),
            _ => None,
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
            Some(columns) => columns.shape,
            None => None,
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

    /// Returns the function that builds the JSON Schema of the whole
    /// document that the leaf prints, as
    /// [`schemas::document_schema`](crate::schemas::document_schema)
    /// builds it, or `None` when the leaf has no document.
    pub(crate) fn envelope(&self) -> Option<fn(Document<()>) -> Schema> {
        self.columns.map(|columns| columns.envelope)
    }

    /// Reports whether the leaf has a document whose row type has columns,
    /// which its `--help` lists. A prose document has none.
    pub(crate) const fn has_columns(&self) -> bool {
        match self.columns {
            Some(columns) => !columns.compact.is_empty() || !columns.long.is_empty(),
            None => false,
        }
    }

    /// Returns the columns of the leaf's row type as its `--help` describes
    /// them: each name of [`Row::COLUMNS`] and then each name of
    /// [`Row::LONG_COLUMNS`], in declared order, or no column when the leaf
    /// has no document.
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
        let compact = columns.compact.iter().map(|name| (name, false));
        let long = columns.long.iter().map(|name| (name, true));
        compact
            .chain(long)
            .map(|(&name, long)| {
                let property = properties
                    .iter()
                    .find(|(field, _)| *field == name)
                    .map(|(_, property)| *property);
                let description = property
                    .and_then(|property| property.get("description"))
                    .and_then(Value::as_str)
                    .map(first_paragraph);
                Column {
                    name,
                    long,
                    property: property.is_some(),
                    empty: property.is_some_and(can_be_empty),
                    description,
                }
            })
            .collect()
    }
}

/// One column of a leaf's row type, as [`Leaf::column_descriptions`] reads
/// it from the row type's JSON Schema.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Column {
    /// The column's name, as [`Row::COLUMNS`] or [`Row::LONG_COLUMNS`]
    /// spells it.
    name: &'static str,
    /// Whether the name comes from [`Row::LONG_COLUMNS`], so `--long` adds
    /// the column.
    long: bool,
    /// Whether the name is a property of the row type's JSON Schema, which
    /// holds one property for each field as the type serializes it.
    property: bool,
    /// Whether the property's type has a value that a problem field reads
    /// as no problem.
    empty: bool,
    /// The first paragraph of the property's description, or `None` when
    /// the property has no description or the name is not a property.
    description: Option<String>,
}

impl Column {
    /// Returns the column's name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Reports whether `--long` adds the column, because its name comes
    /// from [`Row::LONG_COLUMNS`].
    #[must_use]
    pub const fn is_long(&self) -> bool {
        self.long
    }

    /// Reports whether the name is a property of the row type's JSON
    /// Schema. A name that is not a property prints `-` in every row of the
    /// table.
    #[must_use]
    pub const fn is_property(&self) -> bool {
        self.property
    }

    /// Reports whether the column's type has a value that means no
    /// problem when the column is the field that [`Row::PROBLEM`] names:
    /// its JSON Schema type is a string or a boolean, or it can be null.
    /// crumbkit-test's `check_guide` reads it for that field.
    //
    // RUST-10: the type is read from the property itself and from the
    // variants under its `anyOf` or `oneOf`. A field whose type schemars
    // keeps under `$defs`, such as an enum of strings, counts as having no
    // such value. A reader that follows the `$ref` replaces this when a
    // tool names such a field.
    #[must_use]
    pub const fn can_be_empty(&self) -> bool {
        self.empty
    }

    /// Returns the first paragraph of the column's description, with its
    /// lines joined by spaces, or `None` when the column has none.
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

/// One example of a leaf: the words after the leaf's path and a one-line
/// description, such as `--all` and `Print every book`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Example {
    /// The words after the leaf's path, one argument each.
    words: &'static [&'static str],
    /// The one-line description that `--help` prints below the command line.
    about: &'static str,
}

impl Example {
    /// Returns the example that runs the leaf with `words`, the arguments
    /// after its path, described by `about`. An empty `words` runs the leaf
    /// without arguments, and an empty `about` prints the command line
    /// alone.
    #[must_use]
    pub const fn new(words: &'static [&'static str], about: &'static str) -> Self {
        Self { words, about }
    }

    /// Returns the words after the leaf's path.
    #[must_use]
    pub const fn words(&self) -> &'static [&'static str] {
        self.words
    }

    /// Returns the one-line description.
    #[must_use]
    pub const fn about(&self) -> &'static str {
        self.about
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
    /// The row type's [`Row::PROBLEM`].
    problem: Option<&'static str>,
    /// Builds the row type's JSON Schema, whose properties hold the
    /// columns' descriptions.
    schema: fn() -> Schema,
    /// Builds the JSON Schema of the whole document from its id and
    /// description. A stream's schema describes one line.
    envelope: fn(Document<()>) -> Schema,
    /// The shape of the document's result, or `None` for a stream.
    shape: Option<Shape>,
    /// Whether [`Leaf::stream`] set the document.
    stream: bool,
}

/// Reports whether `property`, the JSON Schema of one field, allows a
/// string, a boolean or null, in its own `type` or in the `type` of a
/// variant under its `anyOf` or `oneOf`.
fn can_be_empty(property: &Value) -> bool {
    let mut schemas = vec![property];
    for keyword in ["anyOf", "oneOf"] {
        if let Some(variants) = property.get(keyword).and_then(Value::as_array) {
            schemas.extend(variants);
        }
    }
    let empty = |kind: &Value| matches!(kind.as_str(), Some("string" | "boolean" | "null"));
    schemas.iter().any(|schema| match schema.get("type") {
        Some(Value::Array(kinds)) => kinds.iter().any(empty),
        Some(kind) => empty(kind),
        None => false,
    })
}

/// Returns the JSON Schema of `R`.
fn schema<R: JsonSchema>() -> Schema {
    schemars::schema_for!(R)
}

/// Returns the `Examples:` section of the help of `leaf`, whose command
/// line starts with `text`, the tool's name and the leaf's path, or `None`
/// when the leaf has no examples. `header` styles the heading and `literal`
/// each command line.
///
/// `-h` lists one command line on each line. `--help`, when `long` is set,
/// follows clap's layout of long options: each command line, its
/// description on the next line at clap's indent of 10 spaces, and a blank
/// line between examples.
pub(crate) fn examples(
    leaf: &Leaf,
    text: &str,
    long: bool,
    header: Style,
    literal: Style,
) -> Option<String> {
    if leaf.examples.is_empty() {
        return None;
    }
    let mut section = format!("{header}Examples:{header:#}");
    for (index, example) in leaf.examples.iter().enumerate() {
        if long && index > 0 {
            section.push('\n');
        }
        // Writing to a `String` cannot fail.
        let _ = write!(section, "\n  {literal}{text}");
        for word in example.words {
            section.push(' ');
            section.push_str(&quote(word));
        }
        let _ = write!(section, "{literal:#}");
        if long && !example.about.is_empty() {
            let _ = write!(section, "\n          {}", example.about);
        }
    }
    Some(section)
}

/// Returns the sections of the `--help` of `leaf` that describe its
/// columns: `Columns:` with the row type's [`Row::COLUMNS`] and
/// `Columns that --long adds:` with its [`Row::LONG_COLUMNS`], each in
/// declared order, leaving out a section with no names. It returns no
/// section when the leaf has no document. `header` styles each heading and
/// `literal` each name.
///
/// Each line has the layout of the goal list, and holds the column's name
/// and the description that [`Leaf::column_descriptions`] reads. A column
/// without a description prints its name alone. This builds the schema, so
/// `parse` calls it only for `--help`.
pub(crate) fn columns(leaf: &Leaf, header: Style, literal: Style) -> Vec<String> {
    let columns = leaf.column_descriptions();
    let mut sections = Vec::new();
    for (heading, long) in [("Columns:", false), ("Columns that --long adds:", true)] {
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

/// Returns the first paragraph of `description`, the text before its first
/// blank line, with its lines joined by spaces.
fn first_paragraph(description: &str) -> String {
    description
        .lines()
        .map(str::trim)
        .take_while(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}
