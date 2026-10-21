//! The `Children:` line of a leaf whose rows print their children as lines
//! under them (ADR-0120, ADR-0137).

use super::{Column, Leaf};
use crate::row;

#[cfg(test)]
mod tests;

/// Returns the `Children:` line that prints `text`, the description of the
/// child lines that [`Leaf::children_text`] returns, such as `Children: one
/// indented line under each row for each of its components.`. Help prints
/// this line, and crumbkit-markdown prints it with the `children_help` text
/// of `commands list`, so both print the same label.
#[must_use]
pub fn children_line(text: &str) -> String {
    format!("Children: {text}")
}

/// Returns crumbkit's description of the child lines of rows whose
/// children field is `field` and whose child lines start after `indent`
/// spaces. The sentence calls the lines indented only when `indent` is
/// above 0, because a line at column 1 is not indented.
fn sentence(field: &str, indent: u8) -> String {
    let indented = if indent == 0 { "" } else { "indented " };
    format!("one {indented}line under each row for each of its {field}.")
}

impl Leaf {
    /// Returns the field whose items each row of the leaf's document prints
    /// as lines under it, the row type's
    /// [`Row::CHILDREN`](crate::Row::CHILDREN), or `None` when the leaf has
    /// no document or its row type names no such field.
    #[must_use]
    pub const fn children(&self) -> Option<&'static str> {
        match self.columns {
            Some(columns) => columns.children,
            None => None,
        }
    }

    /// Returns the row type's own description of its child lines,
    /// [`Row::CHILDREN_HELP`](crate::Row::CHILDREN_HELP), or `None` when the
    /// leaf has no document or the row type sets none. A row type may set
    /// it without [`Row::CHILDREN`](crate::Row::CHILDREN), and
    /// crumbkit-test's `check_guide` reads it to fail on that.
    #[must_use]
    pub const fn children_help(&self) -> Option<&'static str> {
        match self.columns {
            Some(columns) => columns.children_help,
            None => None,
        }
    }

    /// Returns the text that help prints after `Children: `, or `None` when
    /// the leaf's row type names no children field. The text is the row
    /// type's [`Leaf::children_help`] when it sets one, and otherwise
    /// crumbkit's sentence, such as `one indented line under each row for
    /// each of its components.`, which leaves out `indented` when the row
    /// type's [`Row::CHILD_INDENT`](crate::Row::CHILD_INDENT) is 0.
    /// `commands list` writes it as `children_help`.
    #[must_use]
    pub fn children_text(&self) -> Option<String> {
        let columns = self.columns?;
        let field = columns.children?;
        Some(
            columns
                .children_help
                .map_or_else(|| sentence(field, columns.child_indent), str::to_owned),
        )
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
    /// builds it from [`Leaf::children_text`], or `None` when the leaf's
    /// row type names no children field. A headed leaf has none, because
    /// its text prints each row's bytes.
    pub(crate) fn children_help_line(&self) -> Option<String> {
        let text = self.children_text()?;
        if self.get_document()?.is_headed() {
            return None;
        }
        Some(children_line(&text))
    }
}
