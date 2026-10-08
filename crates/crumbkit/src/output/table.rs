//! The text table that a result prints by default.

use std::borrow::Cow;
use std::io::{self, Write};

use serde_json::Value;

#[cfg(test)]
mod tests;

/// Writes `rows` to `out` as a table of `columns`.
///
/// The first line is the header, which shows each column name in upper case,
/// and each row follows on its own line. Each column is as wide as its widest
/// cell, two spaces separate the columns, and the last column is not padded,
/// so no line ends in spaces. Each row is a serialized row, and a cell is the
/// row's field with the column's name.
pub(super) fn write(out: &mut impl Write, columns: &[&str], rows: &[Value]) -> io::Result<()> {
    let mut lines: Vec<Vec<Cow<'_, str>>> = Vec::with_capacity(rows.len() + 1);
    lines.push(
        columns
            .iter()
            .map(|name| Cow::Owned(name.to_uppercase()))
            .collect(),
    );
    for row in rows {
        lines.push(columns.iter().map(|name| cell(row.get(name))).collect());
    }

    // RUST-10: the width of a cell is its number of characters, and the
    // padding that `write!` adds counts characters too. This aligns text whose
    // characters each take one terminal column. CJK text and most emoji take
    // two columns and combining marks take none, so a table holding them
    // misaligns. Replace `chars().count()` and the `write!` padding with the
    // unicode-width crate when a tool needs to print such text.
    let mut widths = vec![0; columns.len()];
    for line in &lines {
        for (width, cell) in widths.iter_mut().zip(line) {
            *width = (*width).max(cell.chars().count());
        }
    }

    for line in &lines {
        write_line(out, line, &widths)?;
    }
    Ok(())
}

/// Writes one line of the table: each cell padded to its column's width and
/// followed by two spaces, except the last cell, which is written as it is.
fn write_line(out: &mut impl Write, cells: &[Cow<'_, str>], widths: &[usize]) -> io::Result<()> {
    let last = cells.len().saturating_sub(1);
    for (index, (cell, &width)) in cells.iter().zip(widths).enumerate() {
        if index == last {
            write!(out, "{cell}")?;
        } else {
            write!(out, "{cell:<width$}  ")?;
        }
    }
    writeln!(out)
}

/// Returns the text of one cell. A string shows as it is, except that each
/// control character shows as its escape, such as `\n`, so every row stays on
/// one line. A number or a boolean shows in its JSON form, `null` or a missing
/// field as `-`, and an array or an object as compact JSON.
fn cell(value: Option<&Value>) -> Cow<'_, str> {
    match value {
        None | Some(Value::Null) => Cow::Borrowed("-"),
        Some(Value::String(text)) => escape_controls(text),
        // `Value`'s `Display` writes compact JSON.
        Some(other) => Cow::Owned(other.to_string()),
    }
}

/// Returns `text` with each control character replaced by its
/// `char::escape_default` form, such as `\n`, `\t` or `\u{1b}`. A string
/// without a control character is borrowed unchanged, so it costs no
/// allocation.
pub(super) fn escape_controls(text: &str) -> Cow<'_, str> {
    if !text.chars().any(char::is_control) {
        return Cow::Borrowed(text);
    }
    let mut escaped = String::with_capacity(text.len() + 1);
    for character in text.chars() {
        if character.is_control() {
            escaped.extend(character.escape_default());
        } else {
            escaped.push(character);
        }
    }
    Cow::Owned(escaped)
}
