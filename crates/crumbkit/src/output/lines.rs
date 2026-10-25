//! The text of rows that print their own line in place of the table
//! (ADR-0064).

use std::io;

use super::table::escape_controls;
use super::{children, layout};
use crate::Row;
use crate::view::{Table, View};

/// Reports whether the text of rows of type `R` prints each row's own
/// line: the row type sets [`Row::LINE`], and `view` is the compact text
/// table, so the caller passed no `--json`, `--long` or `--fields`.
pub(super) fn chosen<R: Row>(view: &View) -> bool {
    R::LINE.is_some() && *view == View::Text(Table::Compact)
}

/// Returns the line of `row` with its line feed: the row's own text line
/// with each control character and each Unicode format character escaped,
/// as a table cell escapes them, or one line of compact JSON with
/// every field when the row has no text line. A JSON line holds no control
/// character, because JSON escapes them inside strings. The row's child
/// lines follow the line when its type sets `Row::CHILDREN` (ADR-0120).
pub(super) fn line<R: Row>(row: &R) -> io::Result<String> {
    let mut line = if let Some(text) = row.text_line() {
        escape_controls(&text).into_owned()
    } else {
        let mut json = Vec::new();
        layout::serialize(&mut json, row)?;
        String::from_utf8(json).map_err(io::Error::other)?
    };
    line.push('\n');
    line.push_str(&children::block(row, false));
    Ok(line)
}

/// Returns the line of each of `rows`, as [`line`] builds it. With
/// `spaced`, a line whose row follows a row that printed child lines starts
/// with a blank line, so the blank line prints only when another row
/// follows (ADR-0151).
pub(super) fn spaced<'a, R: Row + 'a>(
    rows: impl Iterator<Item = &'a R>,
    spaced: bool,
) -> io::Result<Vec<String>> {
    let mut after_children = false;
    rows.map(|row| {
        let mut line = line(row)?;
        if spaced && after_children {
            line.insert(0, '\n');
        }
        after_children = holds_child_lines(&line);
        Ok(line)
    })
    .collect()
}

/// Reports whether `line`, as [`line`] builds it, holds child lines after
/// the row's own line. The row's own line holds no line feed of its own,
/// because its text is escaped and JSON escapes line feeds inside strings,
/// so a line feed before the last one starts a child line.
pub(super) fn holds_child_lines(line: &str) -> bool {
    let line = line.strip_prefix('\n').unwrap_or(line);
    line.strip_suffix('\n').unwrap_or(line).contains('\n')
}
