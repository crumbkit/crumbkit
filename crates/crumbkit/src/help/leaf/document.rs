//! [`LeafDocument`], which holds what a leaf knows about the document that
//! its command prints.

use schemars::{JsonSchema, Schema};
use serde_json::Value;

use super::Column;
use crate::help::ViewFlag;
use crate::help::view::{self, Kind, Printed};
use crate::{Children, Document, Line, Problem, Row, Rows, Shape, Stream, row};

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
    /// The field that the row type's [`Row::PROBLEM`] names with
    /// [`Problem::field`].
    problem: Option<&'static str>,
    /// The column of the row type's [`Row::PROBLEM`].
    problem_column: Option<&'static str>,
    /// The form of the row type's [`Row::LINE`].
    line: Option<&'static str>,
    /// The row type's [`Row::CHILDREN`], without its functions.
    children: Option<Children<()>>,
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
            problem: problem_field(<T::Row as Row>::PROBLEM),
            problem_column: problem_column(<T::Row as Row>::PROBLEM),
            line: line_form(<T::Row as Row>::LINE),
            children: untyped_children(<T::Row as Row>::CHILDREN),
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
            problem: problem_field(R::PROBLEM),
            problem_column: problem_column(R::PROBLEM),
            line: line_form(R::LINE),
            children: untyped_children(R::CHILDREN),
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
    // crumbkit-test's `check_help` reads this.
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
    /// lists. A prose document has none. A headed document has some when it
    /// names a column or when its row schema has a `text` or `base64`
    /// property, which `--help` lists among the JSON fields (ADR-0062). For
    /// a headed document that names no column it builds the row schema, so
    /// it is called only while help is rendered.
    pub(crate) fn has_columns(&self) -> bool {
        if !self.compact.is_empty() || !self.long.is_empty() {
            return true;
        }
        self.document.is_headed() && !self.column_descriptions().is_empty()
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

    /// Returns the field that marks a row as reporting a problem, which the
    /// row type's [`Row::PROBLEM`] names with [`Problem::field`], or `None`
    /// when the row type names no such field. The leaf's help prints a
    /// `Problems:` line only for this field.
    #[must_use]
    pub const fn problem(&self) -> Option<&'static str> {
        self.problem
    }

    /// Returns the column that shows whether a row reports a problem, from
    /// the row type's [`Row::PROBLEM`] of either kind, or `None` when the
    /// row type names no problem.
    // crumbkit-test's `check_help` reads this.
    #[doc(hidden)]
    #[must_use]
    pub const fn problem_column(&self) -> Option<&'static str> {
        self.problem_column
    }

    /// Returns the form of the text line that each row prints, from the row
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
    /// `base64` for each that is a property of the row schema (ADR-0062).
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
        let has = |name: &str| properties.iter().any(|(field, _)| *field == name);
        let bytes: Vec<&str> = BYTES_FIELDS
            .into_iter()
            .filter(|name| headed && has(name))
            .collect();
        let compact = self.compact.iter().map(|name| (name, false));
        let long = self.long.iter().map(|name| (name, !headed));
        let bytes = bytes.iter().filter(|name| !self.compact.contains(name));
        compact
            .chain(long)
            .chain(bytes.map(|name| (name, false)))
            .map(|(&name, long)| Column::read(&properties, name, long))
            .collect()
    }

    /// Returns the name of the first field of the row type whose schema
    /// holds both a `text` and a `base64` property, directly, through a
    /// local `$ref` or inside an `anyOf`, `oneOf` or `allOf`, or `None` when
    /// a headed document has no such field or the document is not headed.
    /// Such a field is a `Bytes` or an `Option<Bytes>` that lacks
    /// `#[serde(flatten)]`, so the JSON of the document holds the bytes under
    /// that name. It builds the row type's JSON Schema on each call.
    /// crumbkit-test's `check_help` reads it.
    #[doc(hidden)]
    #[must_use]
    pub fn unflattened_bytes_field(&self) -> Option<String> {
        if !self.document.is_headed() {
            return None;
        }
        let schema = (self.schema)();
        let root = schema.as_value();
        row::properties(&schema)
            .into_iter()
            .find(|(_, property)| holds_bytes_fields(root, property))
            .map(|(name, _)| name.to_owned())
    }

    /// Reports whether the document is headed and its row schema has
    /// properties but lacks a top-level `text` or `base64`, so its JSON does
    /// not hold the bytes. A row schema with no properties is not reported.
    /// It builds the row type's JSON Schema on each call. crumbkit-test's
    /// `check_help` reads it.
    #[doc(hidden)]
    #[must_use]
    pub fn headed_row_lacks_bytes_fields(&self) -> bool {
        if !self.document.is_headed() {
            return false;
        }
        let schema = (self.schema)();
        let properties = row::properties(&schema);
        let has = |name: &str| properties.iter().any(|(field, _)| *field == name);
        !properties.is_empty() && !BYTES_FIELDS.iter().all(|name| has(name))
    }

    /// Returns the settings of the row type's child lines, its
    /// [`Row::CHILDREN`] as [`Children::untyped`] gives them, or `None` when
    /// the row type has none.
    #[must_use]
    pub const fn children(&self) -> Option<Children<()>> {
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

/// The names of the fields that a headed document's JSON holds beside the
/// row's own fields when the row flattens its `Bytes` (ADR-0062).
const BYTES_FIELDS: [&str; 2] = ["text", "base64"];

/// Reports whether `schema` has both bytes properties, directly, through a
/// local `$ref` into `root`, or inside an `anyOf`, `oneOf` or `allOf`.
fn holds_bytes_fields(root: &Value, schema: &Value) -> bool {
    let target = schema
        .get("$ref")
        .and_then(Value::as_str)
        .and_then(|reference| reference.strip_prefix('#'))
        .and_then(|pointer| root.pointer(pointer))
        .unwrap_or(schema);
    let direct = target
        .get("properties")
        .and_then(Value::as_object)
        .is_some_and(|fields| BYTES_FIELDS.iter().all(|name| fields.contains_key(*name)));
    direct
        || ["anyOf", "oneOf", "allOf"].iter().any(|key| {
            target
                .get(key)
                .and_then(Value::as_array)
                .is_some_and(|branches| branches.iter().any(|b| holds_bytes_fields(root, b)))
        })
}

/// Returns the JSON Schema of `R`, as the document's schema describes it.
fn schema<R: JsonSchema>() -> Schema {
    crate::schemas::settings()
        .into_generator()
        .into_root_schema_for::<R>()
}

/// Returns the field that `problem` names with [`Problem::field`].
const fn problem_field<R: ?Sized>(problem: Option<Problem<R>>) -> Option<&'static str> {
    match problem {
        Some(problem) => problem.get_field(),
        None => None,
    }
}

/// Returns the column of `problem`.
const fn problem_column<R: ?Sized>(problem: Option<Problem<R>>) -> Option<&'static str> {
    match problem {
        Some(problem) => Some(problem.get_column()),
        None => None,
    }
}

/// Returns the form of `line`.
const fn line_form<R: ?Sized>(line: Option<Line<R>>) -> Option<&'static str> {
    match line {
        Some(line) => Some(line.get_form()),
        None => None,
    }
}

/// Returns `children` without its functions.
const fn untyped_children<R: ?Sized>(children: Option<Children<R>>) -> Option<Children<()>> {
    match children {
        Some(children) => Some(children.untyped()),
        None => None,
    }
}
