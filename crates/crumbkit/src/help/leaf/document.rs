//! [`LeafDocument`], which holds what a leaf knows about the document that
//! its command prints.

use schemars::{JsonSchema, Schema};

use super::Column;
use crate::help::ViewFlag;
use crate::help::view::{self, Kind, Printed};
use crate::{Children, Document, Row, Rows, Shape, Stream, row};

/// The document that a leaf's command prints and the columns of its row
/// type, as [`Leaf::document`](super::Leaf::document) or
/// [`Leaf::stream`](super::Leaf::stream) records them.
///
/// [`Leaf::get_document`](super::Leaf::get_document) returns it. crumbkit's
/// help, `commands list`, `schemas list` and crumbkit-test's checks read it.
#[derive(Clone, Copy, Debug)]
pub struct LeafDocument {
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
    children: Option<Children>,
    /// Builds the row type's JSON Schema, whose properties hold the
    /// columns' descriptions.
    schema: fn() -> Schema,
    /// Builds the JSON Schema of the whole document from its id and
    /// description. A stream's schema describes one line.
    pub(crate) envelope: fn(Document<()>) -> Schema,
    /// The shape of the document's result, or a stream when
    /// [`Leaf::stream`](super::Leaf::stream) set the document.
    kind: Kind,
    /// The row type's [`Row::WHOLE`].
    whole: bool,
}

impl LeafDocument {
    /// Returns the record of `document`, as
    /// [`Leaf::document`](super::Leaf::document) sets it.
    pub(crate) const fn of<T: Rows>(document: Document<T>) -> Self {
        Self {
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
        }
    }

    /// Returns the record of the stream `document`, as
    /// [`Leaf::stream`](super::Leaf::stream) sets it.
    pub(super) const fn of_stream<R: Row>(document: Document<Stream<R>>) -> Self {
        Self {
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
        }
    }

    /// Returns what the leaf's command prints, as far as the view flags
    /// depend on it.
    pub(super) const fn printed(&self) -> Printed {
        Printed {
            kind: self.kind,
            columns: self.compact,
            long: self.long,
            envelope: self.document.get_envelope().wraps(),
            headed: self.document.is_headed(),
            whole: self.whole,
            listing: self.document.listing_text(),
        }
    }

    /// Reports whether `flag`, passed with `--json` when `json` is true,
    /// adds nothing to the JSON of the leaf's command (ADR-0149).
    ///
    /// `--long` and `--full` add nothing when the JSON already holds the
    /// whole result: for prose and bytes, for a document or stream without
    /// crumbkit's envelope, for a headed document and for rows whose type
    /// sets [`Row::WHOLE`]. `parse`, `finish` and `stream` then accept the
    /// flag with `--json`, also when [`Leaf::offers`](super::Leaf::offers)
    /// reports that the leaf does not offer it, print what `--json` alone
    /// prints and write a note that the flag adds nothing. `--fields` never
    /// adds nothing.
    ///
    /// crumbkit-test's `check_help` reads it, so it is public and hidden
    /// from the documentation.
    #[doc(hidden)]
    #[must_use]
    pub const fn adds_nothing(&self, flag: ViewFlag, json: bool) -> bool {
        view::adds_nothing(&self.printed(), flag, json)
    }

    /// Returns the JSON Schema of the row type. It builds the schema on each
    /// call.
    pub(crate) fn row_schema(&self) -> Schema {
        (self.schema)()
    }

    /// Reports whether the row type has columns, which the leaf's `--help`
    /// lists. A prose document has none. A headed document always has some,
    /// because its `--help` lists `text` and `base64` among the JSON fields
    /// also when its row type names no column.
    pub(crate) const fn has_columns(&self) -> bool {
        self.document.is_headed() || !self.compact.is_empty() || !self.long.is_empty()
    }

    /// Returns the document without its result type.
    #[must_use]
    pub const fn document(&self) -> Document<()> {
        self.document
    }

    /// Reports whether [`Leaf::stream`](super::Leaf::stream) set the
    /// document, so the command prints a stream of rows, one line for each.
    #[must_use]
    pub const fn is_stream(&self) -> bool {
        matches!(self.kind, Kind::Stream)
    }

    /// Reports whether the row type sets [`Row::WHOLE`], so `--json` prints
    /// every field of each row without `--full` (ADR-0065). crumbkit-test's
    /// `check_document_ids` reads it to require an opt-out for such a leaf.
    #[must_use]
    pub const fn rows_whole(&self) -> bool {
        self.whole
    }

    /// Returns the field that marks a row as reporting a problem, the row
    /// type's [`Row::PROBLEM`], or `None` when the row type names no such
    /// field.
    #[must_use]
    pub const fn problem(&self) -> Option<&'static str> {
        self.problem
    }

    /// Returns the form of the text line that each row prints, the row
    /// type's [`Row::LINE`], or `None` when its rows print a table.
    #[must_use]
    pub const fn line(&self) -> Option<&'static str> {
        self.line
    }

    /// Returns the columns whose cells a text table at a terminal never
    /// cuts, the row type's [`Row::UNCUT_COLUMNS`]. crumbkit-test's
    /// `check_help` reads it to fail a name that is not one of the row
    /// type's columns.
    #[must_use]
    pub const fn uncut_columns(&self) -> &'static [&'static str] {
        self.uncut
    }

    /// Returns the [`Shape`] of the result, such as [`Shape::Batch`], or
    /// `None` when [`Leaf::stream`](super::Leaf::stream) set the document.
    /// crumbkit-test's `check_effects` reads it to require that a leaf whose
    /// effects are [`per_item`](crate::help::Effects::per_item) prints a
    /// document that reports each item.
    #[must_use]
    pub const fn shape(&self) -> Option<Shape> {
        match self.kind {
            Kind::Document(shape) => Some(shape),
            Kind::Stream => None,
        }
    }

    /// Returns the JSON Schema of the whole document, as
    /// [`schemas::document_schema`](crate::schemas::document_schema) builds
    /// it. It builds the schema on each call.
    #[must_use]
    pub fn document_schema(&self) -> Schema {
        (self.envelope)(self.document)
    }

    /// Reports whether the row type has any field, as [`Row`] describes the
    /// fields of a row type. It builds the row type's JSON Schema on each
    /// call. crumbkit-test's `check_help` reads it to fail on a row type
    /// that sets [`Row::WHOLE`], names no columns and has fields that could
    /// be columns.
    #[must_use]
    pub fn row_has_fields(&self) -> bool {
        !row::properties(&(self.schema)()).is_empty()
    }

    /// Returns the columns of the row type as its `--help` describes them:
    /// each name of [`Row::COLUMNS`] and then each name of
    /// [`Row::LONG_COLUMNS`], in declared order. A headed document's text
    /// has no columns, so its names are the JSON fields that `--fields`
    /// selects: the same names, none of them long, and then `text` and
    /// `base64` (ADR-0062).
    ///
    /// It builds the row type's JSON Schema, so it costs a schema build on
    /// each call. A column's description is the first paragraph of the
    /// `description` of its property in the schema, which schemars takes
    /// from the field's doc comment. The properties are those that
    /// [`Row`] describes as the fields of a row type, so a column of an
    /// enum's variant has a property too. crumbkit's `--help` and
    /// crumbkit-test's `check_help` both read the columns through this
    /// method.
    //
    // RUST-10: the description comes only from the property itself. A
    // column whose name is not a property, and a field without a doc
    // comment, whose type's doc comment schemars keeps under `$defs`, have
    // no description. crumbkit-test's `check_help` fails on each such
    // column, so a tool finds them in a test. A reader that follows a
    // property's `$ref` into `$defs` replaces this when a tool needs those
    // descriptions.
    #[must_use]
    pub fn column_descriptions(&self) -> Vec<Column> {
        let schema = (self.schema)();
        let properties = row::properties(&schema);
        let headed = self.document.is_headed();
        let bytes: &[&str] = if headed { &["text", "base64"] } else { &[] };
        let compact = self.compact.iter().map(|name| (name, false));
        let long = self.long.iter().map(|name| (name, !headed));
        let bytes = bytes.iter().filter(|name| !self.compact.contains(name));
        compact
            .chain(long)
            .chain(bytes.map(|name| (name, false)))
            .map(|(&name, long)| Column::read(&properties, name, long))
            .collect()
    }

    /// Returns the row type's child lines, its [`Row::CHILDREN`], or `None`
    /// when the row type has none.
    #[must_use]
    pub const fn children(&self) -> Option<Children> {
        self.children
    }

    /// Returns the field that [`Children::field`] names as a [`Column`],
    /// with its property and description read from the row type's JSON
    /// Schema as [`LeafDocument::column_descriptions`] reads a column's, or
    /// `None` when the row type has no child lines. It builds the schema on
    /// each call. crumbkit-test's `check_help` reads it to fail on a field
    /// that is not a property or has no doc comment.
    #[must_use]
    pub fn children_column(&self) -> Option<Column> {
        let children = self.children?;
        let schema = (self.schema)();
        Some(Column::read(
            &row::properties(&schema),
            children.get_field(),
            false,
        ))
    }
}

/// Returns the JSON Schema of `R`, as the document's schema describes it.
fn schema<R: JsonSchema>() -> Schema {
    crate::schemas::settings()
        .into_generator()
        .into_root_schema_for::<R>()
}
