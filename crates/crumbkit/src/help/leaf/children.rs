//! The `Children:` line of a leaf whose rows print their children as
//! indented lines under them (ADR-0120).

use super::{Column, Leaf};
use crate::row;

#[cfg(test)]
mod tests;

/// Returns the `Children:` line that help prints for rows whose row type
/// sets [`Row::CHILDREN`](crate::Row::CHILDREN) to `field`, such as
/// `Children: one indented line under each row for each of its
/// components.`. crumbkit-markdown prints the same line in a reference.
#[must_use]
pub fn children_line(field: &str) -> String {
    format!("Children: one indented line under each row for each of its {field}.")
}

impl Leaf {
    /// Returns the field whose items each row of the leaf's document prints
    /// as indented lines under it, the row type's
    /// [`Row::CHILDREN`](crate::Row::CHILDREN), or `None` when the leaf has
    /// no document or its row type names no such field.
    #[must_use]
    pub const fn children(&self) -> Option<&'static str> {
        match self.columns {
            Some(columns) => columns.children,
            None => None,
        }
    }

    /// Returns the field that [`Leaf::children`] names as a [`Column`],
    /// with its property and description read from the row type's JSON
    /// Schema as [`Leaf::column_descriptions`] reads a column's, or `None`
    /// when the leaf's row type names no such field. It builds the schema
    /// on each call. crumbkit-test's `check_guide` reads it to fail on a
    /// field that is not a property or has no doc comment.
    #[must_use]
    pub fn children_column(&self) -> Option<Column> {
        let columns = self.columns?;
        let field = columns.children?;
        let schema = (columns.schema)();
        Some(Column::read(&row::properties(&schema), field, false))
    }

    /// Returns the `Children:` line that the leaf's `-h` and `--help` print
    /// after the `Output:` line and the `Text:` line, as [`children_line`]
    /// builds it, or `None` when the leaf's row type names no children
    /// field. A headed leaf has none, because its text prints each row's
    /// bytes.
    pub(crate) fn children_help_line(&self) -> Option<String> {
        let field = self.children()?;
        if self.get_document()?.is_headed() {
            return None;
        }
        Some(children_line(field))
    }
}
