//! Leaves, which give a command without subcommands examples in its help
//! and let its `--help` describe each output column.

use std::fmt::Write as _;

use clap::builder::StyledStr;
use clap::builder::styling::Style;
use schemars::{JsonSchema, Schema};
use serde_json::Value;

use super::write_line;
use crate::{Document, Row, Rows, row};

#[cfg(test)]
mod tests;

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

    /// Sets the document that the command prints, the same constant that
    /// the tool passes to [`finish`](crate::finish). The command's `--help`
    /// then lists the columns of the document's row type, each with the doc
    /// comment of its field, and its `-h` points at them.
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
            schema: schema::<T::Row>,
            envelope: crate::schemas::envelope::<T>,
        });
        self
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

    /// Reports whether the leaf has a document, whose columns its `--help`
    /// lists.
    pub(crate) const fn has_document(&self) -> bool {
        self.columns.is_some()
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
    /// Builds the row type's JSON Schema, whose properties hold the
    /// columns' descriptions.
    schema: fn() -> Schema,
    /// Builds the JSON Schema of the whole document from its id and
    /// description.
    envelope: fn(Document<()>) -> Schema,
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

/// Returns `word` as a POSIX shell reads it back as one argument: as it is
/// when it holds only ASCII letters, digits and `-_./:,=+@%`, and otherwise
/// between single quotes, with each `'` in it written as `'\''`. An empty
/// word prints as `''`.
///
/// A leaf's examples, a [`Call`](crate::Call)'s words and the command
/// lines of crumbkit-aliases quote each word with it, so every command
/// line that crumbkit prints quotes in the same way.
///
/// ```
/// use crumbkit::help::quote;
///
/// assert_eq!(quote("--author"), "--author");
/// assert_eq!(quote("Jane Austen"), "'Jane Austen'");
/// ```
//
// RUST-10: the quoting suits POSIX shells, which crumbkit's supported
// platforms use. It misses other shells, such as PowerShell or cmd.exe,
// which read single quotes and backslashes differently. A rendering for
// each shell replaces it if crumbkit supports Windows.
#[must_use]
pub fn quote(word: &str) -> String {
    let plain = !word.is_empty()
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./:,=+@%".contains(c));
    if plain {
        word.to_owned()
    } else {
        format!("'{}'", word.replace('\'', r"'\''"))
    }
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
