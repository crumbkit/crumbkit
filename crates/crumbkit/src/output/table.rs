//! The text table that a result prints by default.

use std::borrow::Cow;
use std::cmp::Ordering;
use std::io::{self, Write};

use serde_json::Value;

mod fit;
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
    /// Whether each column keeps its cells whole when the table fits a
    /// terminal's width, or no entry when every column may be cut.
    uncut: Vec<bool>,
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
            uncut: Vec::new(),
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
    ///
    /// `width` is the number of columns of the terminal that standard
    /// output is, or `None` when the table prints unfitted. A table wider
    /// than `width` narrows its columns and cuts their cells, as
    /// [`fit::fit`] describes.
    pub(super) fn write(
        &mut self,
        out: &mut impl Write,
        rows: usize,
        width: Option<usize>,
    ) -> io::Result<()> {
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
        let header = lines.first().map_or(&[][..], Vec::as_slice);
        let fitted =
            width.is_some_and(|width| fit::fit(&mut self.widths, header, &self.uncut, width));
        for line in lines {
            if fitted {
                fit::write_cut_line(out, line, &self.widths)?;
            } else {
                write_line(out, line, &self.widths)?;
            }
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
/// control character and each Unicode format character shows as its escape,
/// such as `\n` or `\u{202e}`, so every row stays on one line in the order it
/// was written. A number or a boolean shows in its JSON form, `null` or a
/// missing field as `-`, and an array or an object as compact JSON.
pub(crate) fn cell(value: Option<&Value>) -> Cow<'_, str> {
    match value {
        None | Some(Value::Null) => Cow::Borrowed("-"),
        Some(Value::String(text)) => escape_controls(text),
        // `Value`'s `Display` writes compact JSON.
        Some(other) => Cow::Owned(other.to_string()),
    }
}

/// Returns `text` with each control character and each Unicode format
/// character replaced by its `char::escape_default` form, such as `\n`, `\t`,
/// `\u{1b}` or `\u{202e}`. A control character is one of general category Cc,
/// which `char::is_control` reports. A format character is one of general
/// category Cf, such as a bidirectional override or a zero-width space, which
/// can reorder or hide text at a terminal. A string without either is
/// borrowed unchanged, so it costs no allocation.
pub(crate) fn escape_controls(text: &str) -> Cow<'_, str> {
    if !text.chars().any(escaped) {
        return Cow::Borrowed(text);
    }
    let mut escaped_text = String::with_capacity(text.len() + 1);
    for character in text.chars() {
        if escaped(character) {
            escaped_text.extend(character.escape_default());
        } else {
            escaped_text.push(character);
        }
    }
    Cow::Owned(escaped_text)
}

/// Reports whether `escape_controls` replaces `character` with its escape.
fn escaped(character: char) -> bool {
    character.is_control() || is_format(character)
}

/// Reports whether `character` is in Unicode general category Cf, a format
/// character.
fn is_format(character: char) -> bool {
    let point = u32::from(character);
    FORMAT
        .binary_search_by(|&(first, last)| {
            if last < point {
                Ordering::Less
            } else if first > point {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        })
        .is_ok()
}

/// The code-point ranges of general category Cf in Unicode 16.0, sorted, with
/// the first and last code point of each range.
///
/// RUST-10: the standard library has no test for general category Cf, and a
/// crate of Unicode tables would be a dependency for 21 ranges (ADR-0003).
/// This table is updated by hand from `UnicodeData.txt` when a new Unicode
/// version adds a format character. Until then, a format character added
/// after Unicode 16.0 prints unescaped.
const FORMAT: &[(u32, u32)] = &[
    (0x00AD, 0x00AD),
    (0x0600, 0x0605),
    (0x061C, 0x061C),
    (0x06DD, 0x06DD),
    (0x070F, 0x070F),
    (0x0890, 0x0891),
    (0x08E2, 0x08E2),
    (0x180E, 0x180E),
    (0x200B, 0x200F),
    (0x202A, 0x202E),
    (0x2060, 0x2064),
    (0x2066, 0x206F),
    (0xFEFF, 0xFEFF),
    (0xFFF9, 0xFFFB),
    (0x110BD, 0x110BD),
    (0x110CD, 0x110CD),
    (0x13430, 0x1343F),
    (0x1BCA0, 0x1BCA3),
    (0x1D173, 0x1D17A),
    (0xE0001, 0xE0001),
    (0xE0020, 0xE007F),
];
