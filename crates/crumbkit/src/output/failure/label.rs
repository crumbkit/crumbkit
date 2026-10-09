//! The labels of a failure's miette diagnostic. With the `miette` feature, a
//! failure prints each label as an `at:` line in text and as one entry of
//! `labels` in JSON.

use std::io::{self, Write};

use miette::{Diagnostic, LabeledSpan, SourceCode, SourceSpan};
use serde::ser::{Serialize, SerializeMap, Serializer};

use super::super::table::escape_controls;
use super::write_labelled;

#[cfg(test)]
mod tests;

/// One label of a failure's diagnostic: a byte span of the source, what the
/// label says about the span, and where the span starts when the source can
/// be read there.
pub(super) struct Label {
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

/// Where a label's span starts in its source, with the source line that
/// holds that start. The line is held in three parts, without its line
/// break: the text before the span, the text of the span up to the end of
/// the line, and the rest of the line.
struct Place {
    /// The name of the source, such as a file name, when it has one.
    name: Option<String>,
    /// The 1-based number of the line that holds the start of the span.
    line: usize,
    /// The text of the line before the span.
    before: String,
    /// The text of the span, up to the end of the line.
    marked: String,
    /// The text of the line after the span.
    after: String,
}

/// Reads every label of `diagnostic` against its source code, in the order
/// the diagnostic gives them.
pub(super) fn read(diagnostic: &dyn Diagnostic) -> Vec<Label> {
    let Some(labels) = diagnostic.labels() else {
        return Vec::new();
    };
    let source = diagnostic.source_code();
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

    /// Writes the label's `at:` line to `err`. When the span could be read,
    /// the source line that holds its start and a caret underline follow:
    ///
    /// ```text
    ///   at:    shelf.toml:3:1: this key is not known
    ///      3 | colour = "red"
    ///        | ^^^^^^
    /// ```
    ///
    /// The line number is right-aligned in six columns, so the source line
    /// starts in the same column as the values of the labelled lines. A span
    /// that the source cannot read prints as `at:    byte 400` with no
    /// source line.
    pub(super) fn write_text(&self, err: &mut impl Write) -> io::Result<()> {
        let at = match &self.place {
            Some(place) => match &place.name {
                Some(name) => format!("{name}:{}:{}", place.line, place.column()),
                None => format!("{}:{}", place.line, place.column()),
            },
            None => format!("byte {}", self.offset),
        };
        match &self.text {
            Some(text) => write_labelled(err, "at:", &format!("{at}: {text}"))?,
            None => write_labelled(err, "at:", &at)?,
        }
        let Some(place) = &self.place else {
            return Ok(());
        };

        // RUST-10: the underline counts characters of the printed line, so it
        // lines up when each character takes one terminal column. A control
        // character such as a tab prints as its escape, `\t`, and the
        // underline counts the escape. A wide character, such as most CJK
        // text and emoji, takes two columns and a combining mark takes none,
        // so a line holding one misaligns the carets after it. miette's
        // graphical handler, behind miette's `fancy` feature, is the fuller
        // renderer: it measures with unicode-width and draws spans across
        // several lines. A tool that needs it renders the diagnostic itself.
        let before = escape_controls(&place.before);
        let marked = escape_controls(&place.marked);
        let after = escape_controls(&place.after);
        writeln!(err, "{:>6} | {before}{marked}{after}", place.line)?;
        // An empty span, or one that starts at the line break, still gets
        // one caret.
        let carets = "^".repeat(marked.chars().count().max(1));
        let indent = before.chars().count();
        writeln!(err, "{:6} | {:indent$}{carets}", "", "")
    }
}

impl Place {
    /// Reads where `span` starts in `source`, or returns `None` when
    /// `source` cannot read the span, such as a span past its end.
    fn read(source: &dyn SourceCode, span: &SourceSpan) -> Option<Self> {
        // With no context lines, `read_span` returns the bytes of the span,
        // the source's name, the 0-based line that holds the start of the
        // span, and the 0-based column of that start counted in bytes. A
        // `\n`, a `\r\n` and a lone `\r` each end a line.
        let start = source.read_span(span, 0, 0).ok()?;
        let line_start = span.offset().saturating_sub(start.column());
        // An empty span at the start of the line with one context line after
        // it reads from the start of the line to the end of the next line,
        // so the text up to the first line break is the whole line.
        let lines = source
            .read_span(&SourceSpan::new(line_start.into(), 0), 0, 1)
            .ok()?;
        let line = lines
            .data()
            .split(|&byte| byte == b'\n' || byte == b'\r')
            .next()
            .unwrap_or_default();
        let (before, rest) = line.split_at(start.column().min(line.len()));
        let (marked, after) = rest.split_at(span.len().min(rest.len()));
        Some(Self {
            name: start.name().map(str::to_owned),
            line: start.line() + 1,
            before: String::from_utf8_lossy(before).into_owned(),
            marked: String::from_utf8_lossy(marked).into_owned(),
            after: String::from_utf8_lossy(after).into_owned(),
        })
    }

    /// Returns the 1-based column of the start of the span, counted in
    /// characters of the line, as an editor counts them.
    fn column(&self) -> usize {
        self.before.chars().count() + 1
    }
}

/// Serializes the label as an object with `offset` and `length`, then
/// `name`, `line` and `column` when the span could be read, and `text` when
/// the label has text. The source line stays out of the JSON document.
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
            map.serialize_entry("column", &place.column())?;
        }
        if let Some(text) = &self.text {
            map.serialize_entry("text", text)?;
        }
        map.end()
    }
}
