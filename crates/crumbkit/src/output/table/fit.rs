//! Fitting the text table to the width that the sink gives the result,
//! such as the width of the terminal that standard output is (ADR-0121,
//! ADR-0134).

use std::borrow::Cow;
use std::io::{self, Write};

use super::{Table, write_cells};

#[cfg(test)]
mod tests;

impl Table<'_> {
    /// Returns the table with the cells of each column that `uncut` names
    /// kept whole when the table fits a terminal's width, as
    /// [`Row::UNCUT_COLUMNS`](crate::Row::UNCUT_COLUMNS) describes.
    /// `columns` are the names of the table's columns, as [`Table::new`]
    /// took them.
    pub(in crate::output) fn keeping_whole(mut self, columns: &[&str], uncut: &[&str]) -> Self {
        if !uncut.is_empty() {
            self.uncut = columns.iter().map(|name| uncut.contains(name)).collect();
        }
        self
    }
}

/// Narrows `widths`, the width of each column of a table whose header line
/// is `header`, so that a line of the table is at most `width` characters,
/// and reports whether it narrowed any column. `uncut` says for each column
/// whether its cells stay whole, and a column past its end is cut like any
/// other.
///
/// A line is as wide as the sum of the column widths plus two characters
/// for each separator. While a line is wider than `width`, the widest
/// column that is not uncut and is wider than its header narrows by the
/// excess, and never below its header's width. Of two such columns equally
/// wide, the leftmost narrows. When the uncut columns and the headers of
/// the other columns are wider than `width` on their own, no narrowing can
/// make the table fit, so `widths` stay unchanged and the table prints
/// unfitted.
///
/// RUST-10: a width counts characters, as [`Table::write`] does, so a table
/// of text whose characters take two terminal columns, such as CJK text,
/// fits too loosely and still overflows. Measuring with the unicode-width
/// crate replaces the count when the table does.
pub(super) fn fit(
    widths: &mut [usize],
    header: &[Cow<'_, str>],
    uncut: &[bool],
    width: usize,
) -> bool {
    let header_width = |column: usize| header.get(column).map_or(0, |name| name.chars().count());
    let kept = |column: usize| uncut.get(column).copied().unwrap_or(false);
    let separators = 2 * widths.len().saturating_sub(1);
    let line = |widths: &[usize]| widths.iter().sum::<usize>() + separators;
    let narrowest = (0..widths.len())
        .map(|column| {
            if kept(column) {
                widths[column]
            } else {
                header_width(column).min(widths[column])
            }
        })
        .sum::<usize>()
        + separators;
    if line(widths) <= width || narrowest > width {
        return false;
    }
    while line(widths) > width {
        let excess = line(widths) - width;
        let widest = (0..widths.len())
            .filter(|&column| !kept(column) && widths[column] > header_width(column))
            // `max_by` keeps the last of equal columns, so a column to the
            // left counts as wider than an equally wide one to its right.
            .max_by(|&left, &right| widths[left].cmp(&widths[right]).then(right.cmp(&left)));
        let Some(column) = widest else {
            break;
        };
        widths[column] = widths[column]
            .saturating_sub(excess)
            .max(header_width(column));
    }
    true
}

/// Writes one line of a fitted table, as [`write_cells`] writes a line from
/// each cell's text and its column's width, with each cell wider than its
/// column cut to fit by [`cut`].
pub(super) fn write_cut_cells<'c>(
    out: &mut impl Write,
    cells: impl Iterator<Item = (&'c str, usize)>,
) -> io::Result<()> {
    let cut: Vec<(Cow<'c, str>, usize)> = cells
        .map(|(cell, width)| (cut(cell, width), width))
        .collect();
    write_cells(out, cut.iter().map(|(cell, width)| (cell.as_ref(), *width)))
}

/// Returns `cell` when it is at most `width` characters, and otherwise its
/// first `width - 1` characters followed by `…`, so the cut cell is `width`
/// characters and shows that it was cut.
fn cut(cell: &str, width: usize) -> Cow<'_, str> {
    if cell.chars().count() <= width {
        return Cow::Borrowed(cell);
    }
    let mut cut: String = cell.chars().take(width.saturating_sub(1)).collect();
    cut.push('…');
    Cow::Owned(cut)
}
