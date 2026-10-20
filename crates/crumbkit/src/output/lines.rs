//! The text of rows that print their own line in place of the table
//! (ADR-0064).

use std::io;

use super::layout;
use super::table::escape_controls;
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
/// character, because JSON escapes them inside strings.
pub(super) fn line<R: Row>(row: &R) -> io::Result<String> {
    let mut line = if let Some(text) = row.text_line() {
        escape_controls(&text).into_owned()
    } else {
        let mut json = Vec::new();
        layout::serialize(&mut json, row)?;
        String::from_utf8(json).map_err(io::Error::other)?
    };
    line.push('\n');
    Ok(line)
}
