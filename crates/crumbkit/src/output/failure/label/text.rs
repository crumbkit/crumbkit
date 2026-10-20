//! The text form of a failure's labels: an `at:` line for each label, then
//! the source lines around its span with a caret underline, as shelf's golden
//! file [`books-show-from-bad-line.stderr`][bad-line] shows.
//!
//! [bad-line]: https://github.com/crumbkit/crumbkit/blob/main/examples/shelf/tests/golden/books-show-from-bad-line.stderr

use std::io::{self, Write};

use unicode_width::UnicodeWidthStr;

use super::super::super::table::escape_controls;
use super::super::write_labelled;
use super::{Label, Line, Place};

#[cfg(test)]
mod tests;

/// The most lines of one span that print in full. A span over more lines
/// prints its first and its last [`SHOWN_AT_EACH_END`] lines and one row
/// between them that counts the lines left out, so a long span keeps a
/// failure short and still shows where the span starts and ends.
const MOST_SPAN_LINES: usize = 7;

/// The lines at each end of a span over more than [`MOST_SPAN_LINES`] lines
/// that print.
const SHOWN_AT_EACH_END: usize = 3;

/// Writes the text form of `labels` to `err`, in the order the diagnostic
/// gives them.
///
/// Each label prints an `at:` line with its place and text. When its span
/// could be read, the lines that the span covers follow, with one line
/// before and one after where the source has them. A caret underline marks
/// the span on its first and its last line. The line number is
/// right-aligned in six columns, so the source text starts in the same
/// column as the values of the labelled lines. A span that the source cannot
/// read prints as `at:    byte 400` with no source line.
///
/// Labels that follow each other in the diagnostic's order and whose lines
/// overlap or touch print as one block: their `at:` lines first, then each
/// source line once. Each label that marks a line gets its own underline
/// row there, which ends with the label's text, so a reader can tell the
/// labels apart.
pub(in super::super) fn write_text(err: &mut impl Write, labels: &[Label]) -> io::Result<()> {
    let mut start = 0;
    while let Some(first) = labels.get(start) {
        let mut end = start + 1;
        if let Some(place) = &first.place {
            let mut lowest = place.first_printed();
            let mut highest = place.last_printed();
            while let Some(next) = labels.get(end).and_then(|label| label.place.as_ref()) {
                if next.first_printed() > highest + 1 || next.last_printed() + 1 < lowest {
                    break;
                }
                lowest = lowest.min(next.first_printed());
                highest = highest.max(next.last_printed());
                end += 1;
            }
        }
        write_block(err, labels.get(start..end).unwrap_or_default())?;
        start = end;
    }
    Ok(())
}

/// Writes one block: the `at:` line of each label in `labels`, then the
/// source lines that the labels print, with an underline row under a line
/// for each label that marks it.
fn write_block(err: &mut impl Write, labels: &[Label]) -> io::Result<()> {
    for label in labels {
        write_at(err, label)?;
    }
    let places: Vec<&Place> = labels
        .iter()
        .filter_map(|label| label.place.as_ref())
        .collect();
    let (Some(lowest), Some(highest)) = (
        places.iter().map(|place| place.first_printed()).min(),
        places.iter().map(|place| place.last_printed()).max(),
    ) else {
        return Ok(());
    };
    let wanted = |number: usize| places.iter().any(|place| place.wants(number));
    let mut number = lowest;
    while number <= highest {
        if wanted(number) {
            write_line(err, labels, number)?;
            number += 1;
            continue;
        }
        // The last line that a label prints is always wanted, so a run of
        // lines that no label wants ends before `highest`.
        let next = (number..=highest)
            .find(|&next| wanted(next))
            .unwrap_or(highest + 1);
        let left_out = next - number;
        // A row that counts one line takes as much room as the line, so
        // that line prints.
        if left_out == 1 {
            write_line(err, labels, number)?;
        } else {
            writeln!(err, "{:>6} | {left_out} lines left out", "...")?;
        }
        number = next;
    }
    Ok(())
}

/// Writes the `at:` line of `label`: its source's name, its line and column
/// when its span could be read, or its byte offset, then its text.
fn write_at(err: &mut impl Write, label: &Label) -> io::Result<()> {
    let at = match &label.place {
        Some(place) => match &place.name {
            Some(name) => format!("{name}:{}:{}", place.line, place.column),
            None => format!("{}:{}", place.line, place.column),
        },
        None => format!("byte {}", label.offset),
    };
    match &label.text {
        Some(text) => write_labelled(err, "at:", &format!("{at}: {text}")),
        None => write_labelled(err, "at:", &at),
    }
}

/// Writes source line `number`, which a label in `labels` holds, then an
/// underline row for each label that marks the line. A row ends with the
/// label's text when the block holds more than one label. An empty line
/// prints with nothing after its bar, so no line ends in a space.
fn write_line(err: &mut impl Write, labels: &[Label], number: usize) -> io::Result<()> {
    let text = labels
        .iter()
        .find_map(|label| label.place.as_ref()?.line_at(number))
        .map_or("", |line| line.text.as_str());
    if text.is_empty() {
        writeln!(err, "{number:>6} |")?;
    } else {
        writeln!(err, "{number:>6} | {}", escape_controls(text))?;
    }
    for label in labels {
        let Some(line) = label.place.as_ref().and_then(|place| place.line_at(number)) else {
            continue;
        };
        let Some(marked) = &line.marked else {
            continue;
        };
        let before = escape_controls(line.text.get(..marked.start).unwrap_or_default());
        let marked = escape_controls(line.text.get(marked.clone()).unwrap_or_default());
        let indent = width(&before);
        // An empty span, or one that starts at the line break, still gets
        // one caret.
        let carets = "^".repeat(width(&marked).max(1));
        write!(err, "{:6} | {:indent$}{carets}", "", "")?;
        if labels.len() > 1
            && let Some(text) = &label.text
        {
            write!(err, " {}", escape_controls(text))?;
        }
        writeln!(err)?;
    }
    Ok(())
}

/// Returns the number of terminal columns that `text` takes, which places
/// the carets of an underline.
///
/// unicode-width gives each character its width under Unicode's East Asian
/// Width rules, so a wide character such as most CJK text and emoji counts
/// two columns and a combining mark counts none. A control character or a
/// Unicode format character prints as its escape, such as `\t` or `\u{200d}`,
/// before it is measured. An emoji sequence joined by zero-width joiners,
/// such as a family emoji, therefore prints and measures as its separate
/// emoji with an escape between each two.
///
/// RUST-10: unicode-width measures by Unicode's rules, which give an emoji
/// modifier sequence, such as a waving hand with a skin tone, two columns. A terminal whose font has no glyph for the sequence draws each
/// emoji of it on its own, so the sequence shows wider than it measures and
/// the carets after it sit to the left of the labelled text. crumbkit cannot
/// see the terminal's font, so no measure fixes this for every terminal.
fn width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

impl Place {
    /// Returns the number of the first line that the place holds.
    fn first_printed(&self) -> usize {
        self.lines.first().map_or(self.line, |line| line.number)
    }

    /// Returns the number of the last line that the place holds.
    fn last_printed(&self) -> usize {
        self.lines.last().map_or(self.last, |line| line.number)
    }

    /// Returns line `number` when the place holds it.
    fn line_at(&self, number: usize) -> Option<&Line> {
        self.lines.iter().find(|line| line.number == number)
    }

    /// Returns whether line `number` prints for this place: every line that
    /// the place holds, except the middle lines of a span over more than
    /// [`MOST_SPAN_LINES`] lines.
    fn wants(&self, number: usize) -> bool {
        if number < self.first_printed() || number > self.last_printed() {
            return false;
        }
        self.last - self.line < MOST_SPAN_LINES
            || number < self.line + SHOWN_AT_EACH_END
            || number + SHOWN_AT_EACH_END > self.last
    }
}
