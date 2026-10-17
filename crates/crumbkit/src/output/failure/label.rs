//! The labels of a failure's miette diagnostic. With the `miette` feature, a
//! failure prints each label as an `at:` line in text and as one entry of
//! `labels` in JSON, which also holds an excerpt of the source around the
//! label's span.

use std::ops::Range;

use miette::{Diagnostic, LabeledSpan, SourceCode, SourceSpan};
use serde::ser::{Serialize, SerializeMap, Serializer};

#[cfg(test)]
mod tests;
mod text;

pub(super) use self::text::write_text;

/// One label of a failure's diagnostic: a byte span of the source, what the
/// label says about the span, and where the span starts when the source can
/// be read there.
#[derive(Debug)]
pub(crate) struct Label {
    /// The byte offset of the span from the start of the source.
    offset: usize,
    /// The length of the span in bytes.
    length: usize,
    /// What the label says about the span, when it says anything.
    text: Option<String>,
    /// Where the span starts, or `None` when the diagnostic has no source
    /// code or its source code cannot read the span.
    place: Option<Place>,
}

/// Where a label's span starts in its source, with the lines that the span
/// covers and one line before and after them, as the text form prints them,
/// and the excerpt that the JSON document holds.
#[derive(Debug)]
struct Place {
    /// The name of the source, such as a file name, when it has one.
    name: Option<String>,
    /// The 1-based number of the line that holds the start of the span.
    line: usize,
    /// The 1-based column of the start of the span, counted in characters of
    /// its line, as an editor counts them.
    column: usize,
    /// The 1-based number of the last line that the span covers.
    last: usize,
    /// Every line that the span covers, with one line before and one after
    /// where the source has them, in order.
    lines: Vec<Line>,
    /// The same lines as one text, as the JSON document holds them.
    excerpt: Excerpt,
}

/// One line of a source around a label's span, without its line break.
#[derive(Debug)]
struct Line {
    /// The 1-based number of the line.
    number: usize,
    /// The text of the line.
    text: String,
    /// The bytes of `text` that the span marks, on the first and the last
    /// line that the span covers. It is `None` on every other line.
    marked: Option<Range<usize>>,
}

/// The lines of a source around a span: every line the span covers, with one
/// line before and one after where the source has them, as miette's
/// graphical report shows a span by default.
///
/// The excerpt holds whole lines, so a span on a very long line, such as a
/// minified file, brings that whole line into the JSON document.
#[derive(Debug)]
pub(crate) struct Excerpt {
    /// The 1-based number of the first line of the excerpt.
    line: usize,
    /// The lines, joined by the line breaks of the source, without a line
    /// break at the end.
    text: String,
}

/// Reads every label of `diagnostic` against `source`, in the order the
/// diagnostic gives them. `source` is the diagnostic's source code, or for a
/// related diagnostic without one, the source code of the diagnostic it
/// relates to, as miette's graphical report reads it.
pub(super) fn read(diagnostic: &dyn Diagnostic, source: Option<&dyn SourceCode>) -> Vec<Label> {
    let Some(labels) = diagnostic.labels() else {
        return Vec::new();
    };
    labels.map(|label| Label::read(&label, source)).collect()
}

impl Label {
    /// Reads `label` against `source`.
    fn read(label: &LabeledSpan, source: Option<&dyn SourceCode>) -> Self {
        Self {
            offset: label.offset(),
            length: label.len(),
            text: label.label().map(str::to_owned),
            place: source.and_then(|source| Place::read(source, label.inner())),
        }
    }
}

impl Place {
    /// Reads where `span` starts in `source` and the lines around it, or
    /// returns `None` when `source` cannot read the span, such as a span past
    /// its end.
    fn read(source: &dyn SourceCode, span: &SourceSpan) -> Option<Self> {
        // With one context line on each side, `read_span` returns the bytes
        // from the start of the line before the span to the end of the line
        // after it, with that line's break, the source's name, and the
        // 0-based number of the first line it returns.
        let contents = source.read_span(span, 1, 1).ok()?;
        let data = contents.data();
        // The span's start and end, counted in bytes from the start of `data`.
        let start = span.offset().checked_sub(contents.span().offset())?;
        let end = start.saturating_add(span.len());
        let ranges = line_ranges(data);
        // The line that holds the start of the span is the last line that
        // starts at or before it, and the last line that the span covers is
        // the last line that starts before its end. An empty span covers the
        // line that holds it.
        let first = ranges.iter().rposition(|range| range.start <= start)?;
        let last = ranges
            .iter()
            .rposition(|range| range.start < end)
            .map_or(first, |last| last.max(first));
        let mut column = 1;
        let mut lines = Vec::new();
        for (index, range) in ranges.into_iter().enumerate() {
            if index + 1 < first || index > last + 1 {
                continue;
            }
            let bytes = data.get(range.clone()).unwrap_or_default();
            let line = if index == first || index == last {
                // The marked bytes, counted from the start of the line. The
                // span's start is clamped to the line's end, so a span that
                // starts at the line break marks nothing there.
                let from = start.clamp(range.start, range.end) - range.start;
                let to = end.clamp(range.start + from, range.end) - range.start;
                let piece = |range: Range<usize>| {
                    String::from_utf8_lossy(bytes.get(range).unwrap_or_default()).into_owned()
                };
                let before = piece(0..from);
                let marked = piece(from..to);
                if index == first {
                    column = before.chars().count() + 1;
                }
                let range = before.len()..before.len() + marked.len();
                Line {
                    number: contents.line() + index + 1,
                    text: before + &marked + &piece(to..bytes.len()),
                    marked: Some(range),
                }
            } else {
                Line {
                    number: contents.line() + index + 1,
                    text: String::from_utf8_lossy(bytes).into_owned(),
                    marked: None,
                }
            };
            lines.push(line);
        }
        Some(Self {
            name: contents.name().map(str::to_owned),
            line: contents.line() + first + 1,
            column,
            last: contents.line() + last + 1,
            lines,
            excerpt: Excerpt::new(contents.line() + 1, data),
        })
    }
}

/// Returns the byte range of each line of `data`, without its line break. A
/// `\n`, a `\r\n` and a lone `\r` each end a line, as miette counts lines, and
/// a break at the end of `data` starts no line.
fn line_ranges(data: &[u8]) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut start = 0;
    let mut bytes = data.iter().enumerate().peekable();
    while let Some((index, &byte)) = bytes.next() {
        if byte == b'\n' || byte == b'\r' {
            ranges.push(start..index);
            let crlf = byte == b'\r' && bytes.next_if(|&(_, &next)| next == b'\n').is_some();
            start = index + 1 + usize::from(crlf);
        }
    }
    if start < data.len() || ranges.is_empty() {
        ranges.push(start..data.len());
    }
    ranges
}

impl Excerpt {
    /// Returns the excerpt of `data`, whose first line is `line`, without
    /// the line break at its end.
    fn new(line: usize, data: &[u8]) -> Self {
        let data = data
            .strip_suffix(b"\r\n")
            .or_else(|| data.strip_suffix(b"\n"))
            .or_else(|| data.strip_suffix(b"\r"))
            .unwrap_or(data);
        Self {
            line,
            text: String::from_utf8_lossy(data).into_owned(),
        }
    }
}

/// Serializes the excerpt as an object with `line` and `text`.
impl Serialize for Excerpt {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("line", &self.line)?;
        map.serialize_entry("text", &self.text)?;
        map.end()
    }
}

/// Serializes the label as an object with `offset` and `length`, then
/// `name`, `line`, `column` and `excerpt` when the span could be read, and
/// `text` when the label has text.
impl Serialize for Label {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("offset", &self.offset)?;
        map.serialize_entry("length", &self.length)?;
        if let Some(place) = &self.place {
            if let Some(name) = &place.name {
                map.serialize_entry("name", name)?;
            }
            map.serialize_entry("line", &place.line)?;
            map.serialize_entry("column", &place.column)?;
            map.serialize_entry("excerpt", &place.excerpt)?;
        }
        if let Some(text) = &self.text {
            map.serialize_entry("text", text)?;
        }
        map.end()
    }
}
