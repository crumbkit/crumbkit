//! The child lines that a row prints under itself in text (ADR-0120,
//! ADR-0135).

use super::table::escape_controls;
use crate::Row;
use crate::view::{Table, View};

/// Reports whether the text of rows of type `R` prints each row's child
/// lines: the row type sets [`Row::CHILDREN`], and `view` is the text
/// table without `--fields`, so the caller passed no `--json` or
/// `--fields`.
pub(super) fn chosen<R: Row>(view: &View) -> bool {
    R::CHILDREN.is_some() && matches!(view, View::Text(Table::Compact | Table::Long))
}

/// Reports whether the text of rows of type `R` prints a blank line after a
/// row that printed child lines, when another row follows: the row type
/// sets [`Row::SPACED`], and the text prints child lines, as [`chosen`]
/// decides (ADR-0151).
pub(super) fn spaced<R: Row>(view: &View) -> bool {
    R::SPACED && chosen::<R>(view)
}

/// Returns the child lines of `row` as the text prints them under the row:
/// [`Row::long_child_lines`] when `long` is set, which `--long` sets, and
/// [`Row::child_lines`] otherwise, each line after [`Row::CHILD_INDENT`]
/// spaces, with each control character and each Unicode format character
/// escaped as a table cell escapes them, and followed by a line feed. It
/// returns an empty string, which does not allocate, when the row type sets
/// no [`Row::CHILDREN`] or the row has no child lines.
pub(super) fn block<R: Row>(row: &R, long: bool) -> String {
    if R::CHILDREN.is_none() {
        return String::new();
    }
    let lines = if long {
        row.long_child_lines()
    } else {
        row.child_lines()
    };
    let mut block = String::new();
    for line in lines {
        block.extend(std::iter::repeat_n(' ', usize::from(R::CHILD_INDENT)));
        block.push_str(&escape_controls(&line));
        block.push('\n');
    }
    block
}

/// Returns the child lines of each of `rows`, as [`block`] builds them, when
/// `view` prints them, and an empty list otherwise.
pub(super) fn blocks<'a, R: Row + 'a>(
    view: &View,
    rows: impl Iterator<Item = &'a R>,
) -> Vec<String> {
    if chosen::<R>(view) {
        let long = view.is_long();
        rows.map(|row| block(row, long)).collect()
    } else {
        Vec::new()
    }
}

/// Returns the names of the fields that the JSON of rows of type `R` holds
/// without `--full` and `--fields` after `columns`: the field that
/// [`Row::CHILDREN`] names, so `--json` carries the children with each row,
/// or nothing when the type names none or `columns` already holds it.
pub(super) fn json_field<R: Row>(columns: &[&str]) -> Option<&'static str> {
    R::CHILDREN.filter(|field| !columns.contains(field))
}
