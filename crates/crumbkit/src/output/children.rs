//! The child lines that a row prints under itself in text (ADR-0120).

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

/// Returns the child lines of `row` as the text prints them under the row:
/// each line indented by two spaces, with each control character and each
/// Unicode format character escaped as a table cell escapes them, and
/// followed by a line feed. It returns an empty string, which does not
/// allocate, when the row type sets no [`Row::CHILDREN`] or the row has no
/// child lines.
pub(super) fn block<R: Row>(row: &R) -> String {
    if R::CHILDREN.is_none() {
        return String::new();
    }
    let mut block = String::new();
    for line in row.child_lines() {
        block.push_str("  ");
        block.push_str(&escape_controls(&line));
        block.push('\n');
    }
    block
}

/// Returns the child lines of each of `rows`, as [`block`] builds them, when
/// `view` prints them, and an empty list otherwise.
pub(super) fn blocks<R: Row>(view: &View, rows: &[R]) -> Vec<String> {
    if chosen::<R>(view) {
        rows.iter().map(block).collect()
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
