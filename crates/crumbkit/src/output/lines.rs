//! The text of rows that print their own line in place of the table
//! (ADR-0064).

use std::io;

use super::Output;
use super::layout::{self, KeyOrder};
use super::table::escape_controls;
use crate::Row;

/// Reports whether the text of rows of type `R` prints each row's own
/// line: the row type sets [`Row::LINE`], the caller did not pass `--json`,
/// and neither `--long` nor `--fields` names columns.
pub(super) fn chosen<R: Row>(output: &Output) -> bool {
    R::LINE.is_some() && !output.json && !output.long && output.fields.is_empty()
}

/// Returns the line of `row` with its line feed: the row's own text line
/// with each control character escaped, or one line of compact JSON with
/// every field, its keys in the order `keys` names, when the row has no
/// text line. A JSON line holds no control character, because JSON escapes
/// them inside strings.
pub(super) fn line<R: Row>(row: &R, keys: Option<KeyOrder>) -> io::Result<String> {
    let mut line = if let Some(text) = row.text_line() {
        escape_controls(&text).into_owned()
    } else {
        let mut json = Vec::new();
        layout::serialize(&mut json, row, keys)?;
        String::from_utf8(json).map_err(io::Error::other)?
    };
    line.push('\n');
    Ok(line)
}
