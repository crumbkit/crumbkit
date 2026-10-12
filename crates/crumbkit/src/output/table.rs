//! The text table that a result prints by default.

use std::borrow::Cow;
use std::io::{self, Write};

use serde_json::Value;

#[cfg(test)]
mod tests;

/// A text table of rows: each column's name in upper case as the header,
/// and the cells of each row, which `finish` builds once and can then
/// measure and write with any number of its first rows.
pub(super) struct Table<'a> {
    /// The header line and then one line of cells for each row.
    lines: Vec<Vec<Cow<'a, str>>>,
    /// The width of each column over the lines last written, kept so that
    /// writing the table again allocates nothing.
    widths: Vec<usize>,
}

impl<'a> Table<'a> {
    /// Returns the table of `rows` with `columns`. Each row is a serialized
    /// row, and a cell is the row's field with the column's name.
    pub(super) fn new(columns: &[&str], rows: &'a [Value]) -> Self {
        let mut lines: Vec<Vec<Cow<'a, str>>> = Vec::with_capacity(rows.len() + 1);
        lines.push(
            columns
                .iter()
                .map(|name| Cow::Owned(name.to_uppercase()))
                .collect(),
        );
        for row in rows {
            lines.push(columns.iter().map(|name| cell(row.get(name))).collect());
        }
        Self {
            lines,
            widths: vec![0; columns.len()],
        }
    }

    /// Writes the header and the first `rows` rows to `out`.
    ///
    /// The first line is the header, which shows each column name in upper
    /// case, and each row follows on its own line. Each column is as wide as
    /// its widest cell among the lines written, and two spaces separate the
    /// columns. A line ends with the text of its last non-empty cell, so no
    /// line ends in padding or separator spaces. A cell's own text prints
    /// unchanged, including any spaces at its end.
    pub(super) fn write(&mut self, out: &mut impl Write, rows: usize) -> io::Result<()> {
        let lines = self.lines.get(..=rows).unwrap_or(&self.lines);
        // RUST-10: the width of a cell is its number of characters, and the
        // padding that `write!` adds counts characters too. This aligns text
        // whose characters each take one terminal column. CJK text and most
        // emoji take two columns and combining marks take none, so a table
        // holding them misaligns. Replace `chars().count()` and the `write!`
        // padding with the unicode-width crate when a tool needs to print
        // such text.
        self.widths.fill(0);
        for line in lines {
            for (width, cell) in self.widths.iter_mut().zip(line) {
                *width = (*width).max(cell.chars().count());
            }
        }
        for line in lines {
            write_line(out, line, &self.widths)?;
        }
        Ok(())
    }
}

/// Writes one line of the table. Each cell starts at its column's offset,
/// which is the end of the cell before it plus that cell's padding to its
/// column's width plus two separator spaces. Those spaces are held back and
/// written only before a non-empty cell, so a line never ends in them. A cell
/// wider than its column gets no padding and moves the rest of its line
/// right, which happens only in a stream, whose widths come from the header.
pub(super) fn write_line(
    out: &mut impl Write,
    cells: &[Cow<'_, str>],
    widths: &[usize],
) -> io::Result<()> {
    let mut spaces = 0;
    for (cell, &width) in cells.iter().zip(widths) {
        if !cell.is_empty() {
            // An empty string padded to `spaces` characters writes that many
            // spaces.
            write!(out, "{:spaces$}{cell}", "")?;
            spaces = 0;
        }
        spaces += width.saturating_sub(cell.chars().count()) + 2;
    }
    writeln!(out)
}

/// Returns the text of one cell. A string shows as it is, except that each
/// control character shows as its escape, such as `\n`, so every row stays on
/// one line. A number or a boolean shows in its JSON form, `null` or a missing
/// field as `-`, and an array or an object as compact JSON.
pub(crate) fn cell(value: Option<&Value>) -> Cow<'_, str> {
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
pub(crate) fn escape_controls(text: &str) -> Cow<'_, str> {
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
