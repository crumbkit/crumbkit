//! The labels of a failure's miette diagnostic. With the `miette` feature, a
//! failure prints each label as an `at:` line in text and as one entry of
//! `labels` in JSON, which also holds an excerpt of the source around the
//! label's span.

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
#[derive(Debug)]
pub struct Label {
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
/// holds that start and the excerpt around the span. The line is held in
/// three parts, without its line break: the text before the span, the text
/// of the span up to the end of the line, and the rest of the line.
#[derive(Debug)]
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
    /// The lines that the span covers with one line before and after, or
    /// `None` when the source cannot read them.
    excerpt: Option<Excerpt>,
}

/// The lines of a source around a span: every line the span covers, with one
/// line before and one after where the source has them, as miette's
/// graphical report shows a span by default.
///
/// The excerpt holds whole lines, so a span on a very long line, such as a
/// minified file, brings that whole line into the JSON document.
#[derive(Debug)]
pub struct Excerpt {
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
    /// Returns the byte offset of the start of the span from the start of
    /// the source.
    #[must_use]
    pub const fn offset(&self) -> usize {
        self.offset
    }

    /// Returns the length of the span in bytes.
    #[must_use]
    pub const fn length(&self) -> usize {
        self.length
    }

    /// Returns what the label says about the span, when it says anything.
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    /// Returns the name of the source, such as a file name, when the span
    /// could be read and the source has a name.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.place.as_ref()?.name.as_deref()
    }

    /// Returns the 1-based line that holds the start of the span, when the
    /// span could be read.
    #[must_use]
    pub fn line(&self) -> Option<usize> {
        self.place.as_ref().map(|place| place.line)
    }

    /// Returns the 1-based column of the start of the span, counted in
    /// characters of its line, when the span could be read.
    #[must_use]
    pub fn column(&self) -> Option<usize> {
        self.place.as_ref().map(Place::column)
    }

    /// Returns the lines of the source around the span, when they could be
    /// read.
    #[must_use]
    pub fn excerpt(&self) -> Option<&Excerpt> {
        self.place.as_ref()?.excerpt.as_ref()
    }

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
    /// the source line that holds its start and a caret underline follow, as
    /// shelf's golden file [`books-show-from-bad-line.stderr`][bad-line]
    /// shows.
    ///
    /// The line number is right-aligned in six columns, so the source line
    /// starts in the same column as the values of the labelled lines. A span
    /// that the source cannot read prints as `at:    byte 400` with no
    /// source line.
    ///
    /// [bad-line]: https://github.com/crumbkit/crumbkit/blob/main/examples/shelf/tests/golden/books-show-from-bad-line.stderr
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
            excerpt: Excerpt::read(source, span),
        })
    }

    /// Returns the 1-based column of the start of the span, counted in
    /// characters of the line, as an editor counts them.
    fn column(&self) -> usize {
        self.before.chars().count() + 1
    }
}

impl Excerpt {
    /// Returns the 1-based number of the first line of the excerpt.
    #[must_use]
    pub const fn line(&self) -> usize {
        self.line
    }

    /// Returns the lines, joined by the line breaks of the source, without a
    /// line break at the end.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Reads the lines around `span` in `source`, or returns `None` when
    /// `source` cannot read them.
    fn read(source: &dyn SourceCode, span: &SourceSpan) -> Option<Self> {
        // With one context line on each side, `read_span` returns the bytes
        // from the start of the line before the span to the end of the line
        // after it, with that line's break, and the 0-based number of the
        // first line it returns.
        let contents = source.read_span(span, 1, 1).ok()?;
        let data = contents.data();
        let data = data
            .strip_suffix(b"\r\n")
            .or_else(|| data.strip_suffix(b"\n"))
            .or_else(|| data.strip_suffix(b"\r"))
            .unwrap_or(data);
        Some(Self {
            line: contents.line() + 1,
            text: String::from_utf8_lossy(data).into_owned(),
        })
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
            map.serialize_entry("column", &place.column())?;
            if let Some(excerpt) = &place.excerpt {
                map.serialize_entry("excerpt", excerpt)?;
            }
        }
        if let Some(text) = &self.text {
            map.serialize_entry("text", text)?;
        }
        map.end()
    }
}
