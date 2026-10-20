//! `Column`, one column of a leaf's row type as its `--help` describes it.

use serde_json::Value;

#[cfg(doc)]
use super::Leaf;
#[cfg(doc)]
use crate::Row;

/// One column of a leaf's row type, as [`Leaf::column_descriptions`] reads
/// it from the row type's JSON Schema.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Column {
    /// The column's name, as [`Row::COLUMNS`] or [`Row::LONG_COLUMNS`]
    /// spells it.
    pub(super) name: &'static str,
    /// Whether the name comes from [`Row::LONG_COLUMNS`], so `--long` adds
    /// the column.
    pub(super) long: bool,
    /// Whether the name is a property of the row type's JSON Schema, which
    /// holds one property for each field as the type serializes it.
    pub(super) property: bool,
    /// Whether the property's type has a value that a problem field reads
    /// as no problem.
    pub(super) empty: bool,
    /// The first paragraph of the property's description, or `None` when
    /// the property has no description or the name is not a property.
    pub(super) description: Option<String>,
}

impl Column {
    /// Returns the column `name` as `properties`, the fields of a row type's
    /// JSON Schema, describe it. `long` says whether the name comes from
    /// [`Row::LONG_COLUMNS`].
    pub(super) fn read(properties: &[(&str, &Value)], name: &'static str, long: bool) -> Self {
        let property = properties
            .iter()
            .find(|(field, _)| *field == name)
            .map(|(_, property)| *property);
        let description = property
            .and_then(|property| property.get("description"))
            .and_then(Value::as_str)
            .map(first_paragraph);
        Self {
            name,
            long,
            property: property.is_some(),
            empty: property.is_some_and(crate::row::can_be_empty),
            description,
        }
    }

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
    /// Schema. A name that is not a property is missing in every row, so the
    /// table leaves its column out.
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
