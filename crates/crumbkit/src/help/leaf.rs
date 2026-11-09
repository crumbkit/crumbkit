//! Leaves, which give a command without subcommands examples in its help
//! and let its `--help` describe each output column.

use clap::builder::StyledStr;
use clap::builder::styling::Style;

use super::view::{self, Unoffered, ViewFlag};
use super::{Effects, write_line};
use crate::pick::Choices;
use crate::{Call, Document, Row, Rows, Shape, Stream};

mod children;
mod column;
mod document;
mod example;
mod lines;
mod no_result;
mod passes;
mod refusal;
#[cfg(test)]
mod tests;
mod text;

pub use children::children_line;
pub use column::Column;
pub use document::LeafDocument;
pub use example::Example;
pub(crate) use example::examples;
pub use lines::{BYTES_LINE, HEADED_LINE, STREAM_LINE};
pub use refusal::RefusalContext;
pub use text::text_line;

/// A command without subcommands whose help shows examples, and whose
/// `--help` describes the columns of the result it prints.
///
/// A tool lists its leaves in its [`Tool`](super::Tool):
///
/// ```
/// # #[derive(serde::Serialize, schemars::JsonSchema)]
/// # struct Book { id: u32 }
/// # impl crumbkit::Row for Book { const COLUMNS: &'static [&'static str] = &["id"]; }
/// use crumbkit::{Document, Tool, help::{Example, Help, Leaf}};
///
/// const BOOKS_LIST: Document<Vec<Book>> =
///     Document::new("shelf", "books-list", 1, "The books in the catalogue.");
///
/// const LEAVES: &[Leaf] = &[Leaf::new(&["books", "list"])
///     .examples(&[
///         Example::new(&[], "Print the first page of books"),
///         Example::new(&["--all"], "Print every book"),
///     ])
///     .document(BOOKS_LIST)];
/// const TOOL: Tool = Tool::new().help(Help::new().leaves(LEAVES));
///
/// assert_eq!(TOOL.get_help().get_leaves()[0].path(), ["books", "list"]);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Leaf {
    /// The names of the subcommands from the root to the command.
    path: &'static [&'static str],
    /// The examples, in the order the help page lists them.
    examples: &'static [Example],
    /// The document that the command prints and the columns of its rows,
    /// or `None` when the leaf has no document.
    document: Option<LeafDocument>,
    /// The command's effects, or `None` when the tool declares none.
    effects: Option<Effects>,
    /// The sources of the values of the command's arguments, which a picker
    /// offers when an argument is missing.
    choices: &'static [Choices],
    /// Whether [`Leaf::prints_no_result`] declared that the command prints
    /// no result.
    no_result: bool,
    /// The program whose exit code the command passes on, or `None` when
    /// the command exits as the tool record's exits declare.
    passes_on_exit: Option<&'static str>,
    /// The function that builds the calls that a refusal of a command line
    /// that names the command offers as its recovery and alternatives.
    refusal_alternatives: Option<fn(&RefusalContext<'_>) -> Vec<Call>>,
    /// The function that returns the hint that a refusal of a command line
    /// that names the command offers as its recovery.
    refusal_hint: Option<fn(&RefusalContext<'_>) -> Option<String>>,
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
            document: None,
            effects: None,
            choices: &[],
            no_result: false,
            passes_on_exit: None,
            refusal_alternatives: None,
            refusal_hint: None,
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
    /// effects unless the tool opts out with a reason, and its `check_help`
    /// fails on a leaf whose effects change something and that has no
    /// document, unless the leaf passes on a program's exit code or prints
    /// no result.
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
    /// tool record's [`Tool::picker`](super::Tool::picker) offers when the
    /// caller leaves a required argument out (ADR-0043). An argument with
    /// possible values offers them and never calls its source.
    /// crumbkit-test's `check_help` fails on a source whose argument id
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
    /// The leaf keeps the document's id and description, the row type's column
    /// lists, and functions that build the JSON Schemas of the row type and of
    /// the whole document, so one tool record holds leaves whose rows have
    /// different types. [`schemas`](crate::schemas) serves each leaf's document
    /// from them.
    #[must_use]
    pub const fn document<T: Rows>(mut self, document: Document<T>) -> Self {
        self.document = Some(LeafDocument::of(document));
        self
    }

    /// Returns the leaf with `document`, or without a document when it is
    /// `None`. A runtime leaf's document reaches its leaf in the joined tool
    /// record this way.
    pub(crate) const fn with_document(mut self, document: Option<LeafDocument>) -> Self {
        self.document = document;
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
        self.document = Some(LeafDocument::of_stream(document));
        self
    }

    /// Returns the `Output:` line that the leaf's `-h` and `--help` print
    /// after the effects line: [`STREAM_LINE`] for a stream,
    /// [`BYTES_LINE`] for a document that holds [`Bytes`](crate::Bytes),
    /// [`HEADED_LINE`] for a headed document, a line that says the command
    /// prints no result for a leaf that [`Leaf::prints_no_result`] declared,
    /// and `None` for any other leaf.
    pub(crate) const fn output_line(&self) -> Option<&'static str> {
        match self.document {
            None if self.no_result => Some(no_result::NO_RESULT_LINE),
            Some(document) if document.is_stream() => Some(STREAM_LINE),
            Some(document) if matches!(document.shape(), Some(Shape::Bytes)) => Some(BYTES_LINE),
            Some(document) if document.document().is_headed() => Some(HEADED_LINE),
            _ => None,
        }
    }

    /// Reports whether the leaf's command offers the view flag `flag`, as
    /// its declared document decides (ADR-0016). A prose or bytes document
    /// offers no view flag. Rows offer `--fields`, offer `--long` when the
    /// row type has long columns, the document lists only the rows that
    /// report a problem (ADR-0138) or it declares a long note (ADR-0139),
    /// and offer `--full` when the document has
    /// crumbkit's envelope. A headed document offers neither `--long` nor
    /// `--full` (ADR-0062). Rows that print whole offer no `--full`, and
    /// offer neither `--fields` nor `--long` when they have no columns
    /// (ADR-0065). `parse` still accepts `--long` and `--full` with `--json`
    /// where the JSON already holds the whole result, and `finish` and
    /// `stream` print a note that the flag adds nothing (ADR-0149). A leaf that [`Leaf::prints_no_result`] declared offers
    /// none (ADR-0143). Any other leaf without a document offers every view
    /// flag, because crumbkit cannot tell what its command prints.
    #[must_use]
    pub const fn offers(&self, flag: ViewFlag) -> bool {
        self.unoffered(flag).is_none()
    }

    /// Returns why the leaf's command does not offer `flag`, or `None` when
    /// it offers it, as [`Leaf::offers`] describes.
    pub(crate) const fn unoffered(&self, flag: ViewFlag) -> Option<Unoffered> {
        match self.document {
            Some(document) => view::unoffered(&document.printed(), flag),
            None if self.no_result => Some(Unoffered::NoResult),
            None => None,
        }
    }

    /// Returns the document and row-type facts that [`Leaf::document`] or
    /// [`Leaf::stream`] set, or `None` when the leaf has no document. The
    /// name has clap's `get_` prefix, because `document` is the builder
    /// method.
    #[must_use]
    pub const fn get_document(&self) -> Option<&LeafDocument> {
        self.document.as_ref()
    }
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
/// and the description that [`LeafDocument::column_descriptions`] reads. A column
/// without a description prints its name alone. This builds the schema, so
/// `parse` calls it only for `--help`.
pub(crate) fn columns(leaf: &Leaf, header: Style, literal: Style) -> Vec<String> {
    let columns = leaf
        .get_document()
        .map_or_else(Vec::new, LeafDocument::column_descriptions);
    let mut sections = Vec::new();
    let compact = match leaf.document {
        Some(document) if document.document().is_headed() => "JSON fields that --fields selects:",
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
