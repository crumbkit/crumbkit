//! The style of the JSON that a tool prints: one line by default, or
//! indented when the tool's guide sets [`JsonLayout::Pretty`] (ADR-0052).
//! The keys of each object keep the order that the value serializes them
//! in, or print sorted when the style sets `sorted_keys` (ADR-0067). The
//! guide sets both with one [`JsonStyle`] (ADR-0073).

use std::io::{self, Write};

use serde::Serialize;

/// The layout of a whole JSON document, which a tool sets in its
/// [`JsonStyle`].
///
/// A stream prints one document per line with any layout, because JSON
/// Lines needs it.
///
/// The enum is `#[non_exhaustive]`, so crumbkit can add a layout without
/// breaking a tool that matches on it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum JsonLayout {
    /// The whole document on one line, whether or not standard output is a
    /// terminal. It is the default.
    #[default]
    OneLine,
    /// Each field and each item on a line of its own, indented by two
    /// spaces for each level, as `serde_json::to_writer_pretty` writes it.
    /// `reason` is why the tool's readers need it, which crumbkit-test's
    /// opt-out report shows with its cost.
    Pretty {
        /// Why the tool's readers need indented JSON.
        reason: &'static str,
    },
}

/// How a tool writes JSON on standard output, which it sets with
/// [`Guide::json`](crate::help::Guide::json) (ADR-0073).
///
/// The default, which [`JsonStyle::new`] returns, prints each document on
/// one line with the keys of each object in the order that its value
/// serializes them: crumbkit's envelope prints the type key first, and a
/// row prints its fields in their declaration order.
///
/// ```
/// use crumbkit::{JsonLayout, JsonStyle};
///
/// const STYLE: JsonStyle = JsonStyle::new().pretty("Our scripts read indented JSON.").sorted();
/// assert!(matches!(STYLE.layout, JsonLayout::Pretty { .. }));
/// assert!(STYLE.sorted_keys);
/// ```
///
/// The fields are public so a tool and crumbkit-test can read them. The
/// struct is `#[non_exhaustive]`, so a tool builds it with its constructors
/// and crumbkit can add a field without breaking the tool.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct JsonStyle {
    /// The layout of a whole document.
    pub layout: JsonLayout,
    /// Whether every object at every depth prints its keys in the order of
    /// Rust's `str` comparison, which compares the keys' Unicode code
    /// points. It is a declaration, so it takes no reason.
    pub sorted_keys: bool,
}

impl JsonStyle {
    /// Returns the default style: one line, with the keys of each object
    /// in the order that its value serializes them.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            layout: JsonLayout::OneLine,
            sorted_keys: false,
        }
    }

    /// Returns the style with [`JsonLayout::Pretty`] and the reason that
    /// the tool's readers need it.
    #[must_use]
    pub const fn pretty(self, reason: &'static str) -> Self {
        Self {
            layout: JsonLayout::Pretty { reason },
            sorted_keys: self.sorted_keys,
        }
    }

    /// Returns the style with the keys of every object sorted.
    #[must_use]
    pub const fn sorted(self) -> Self {
        Self {
            layout: self.layout,
            sorted_keys: true,
        }
    }

    /// Returns the same style on one line, as a stream and the line of a
    /// headed answer need it.
    pub(crate) const fn one_line(self) -> Self {
        Self {
            layout: JsonLayout::OneLine,
            sorted_keys: self.sorted_keys,
        }
    }
}

/// Writes `value` to `out` as JSON in the layout and key order of `style`,
/// and ends it with a line feed.
///
/// Both layouts serialize `value` on one line through `&mut dyn Write`, so
/// the compiler generates the serialization code of each document once,
/// whichever writer receives it. `JsonLayout::Pretty` serializes into a
/// vector and re-indents those bytes with [`indent`] before it writes them,
/// so the layout does not make every tool's binary larger, and `out`
/// receives the indented bytes, so the output budget counts them.
///
/// A serialization error becomes an `io::Error` of kind `InvalidData`, so it
/// is reported as a failed write.
pub(super) fn write(
    out: &mut impl Write,
    value: &(impl Serialize + ?Sized),
    style: JsonStyle,
) -> io::Result<()> {
    match style.layout {
        JsonLayout::Pretty { .. } => {
            let mut compact = Vec::new();
            serialize(&mut compact, value, style.sorted_keys)?;
            write_indented(out, &compact)
        }
        JsonLayout::OneLine => {
            serialize(out, value, style.sorted_keys)?;
            out.write_all(b"\n")
        }
    }
}

/// Writes `value` to `out` on one line, with the keys of each object
/// sorted when `sorted_keys` is set.
///
/// With `sorted_keys`, it serializes `value` into a
/// `serde_json::Value` and sorts every object of it before it writes it.
// RUST-10: sorting through a `Value` builds one tree for each document
// written, and a cut that measures a document several times builds one for
// each measurement. A `Value` holds no integer outside the ranges of `i64`
// and `u64`, so such a number fails the write as `InvalidData`, and of two
// equal keys in one object only the last prints. A serializer that sorts
// while it writes would avoid all three, and no reader has needed it yet.
pub(super) fn serialize(
    out: &mut dyn Write,
    value: &(impl Serialize + ?Sized),
    sorted_keys: bool,
) -> io::Result<()> {
    if sorted_keys {
        let mut sorted = serde_json::to_value(value)?;
        sorted.sort_all_objects();
        Ok(serde_json::to_writer(out, &sorted)?)
    } else {
        Ok(serde_json::to_writer(out, value)?)
    }
}

/// Writes `compact` to `out` re-indented by [`indent`], followed by a line
/// feed.
fn write_indented(out: &mut dyn Write, compact: &[u8]) -> io::Result<()> {
    let mut indented = Vec::with_capacity(compact.len() * 2);
    indent(compact, &mut indented);
    indented.push(b'\n');
    out.write_all(&indented)
}

/// Appends `compact`, JSON as `serde_json::to_writer` writes it, to `out`
/// laid out as `serde_json::to_writer_pretty` writes it.
///
/// It starts a new line, indented by two spaces for each open object or
/// array, after `{`, `[` and `,`, and before `}` and `]`, and writes `: `
/// after a key. An empty object or array stays `{}` or `[]`. Bytes inside
/// a string, including an escaped quote, are copied unchanged. The compact
/// form has no whitespace outside strings, so the result matches `serde_json`'s
/// pretty form byte for byte.
pub(super) fn indent(compact: &[u8], out: &mut Vec<u8>) {
    indent_at(compact, 0, out);
}

/// Appends `compact` to `out` as [`indent`] does for a value that sits
/// `depth` levels deep in a document, so each of its lines is indented as
/// it is inside that document.
pub(super) fn indent_at(compact: &[u8], depth: usize, out: &mut Vec<u8>) {
    let mut depth = depth;
    let mut in_string = false;
    let mut escaped = false;
    let mut bytes = compact.iter().copied().peekable();
    while let Some(byte) = bytes.next() {
        if in_string {
            out.push(byte);
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match byte {
            b'"' => {
                in_string = true;
                out.push(byte);
            }
            b'{' | b'[' => {
                out.push(byte);
                if let Some(close @ (b'}' | b']')) = bytes.peek().copied() {
                    out.push(close);
                    bytes.next();
                } else {
                    depth += 1;
                    new_line(out, depth);
                }
            }
            b'}' | b']' => {
                depth = depth.saturating_sub(1);
                new_line(out, depth);
                out.push(byte);
            }
            b',' => {
                out.push(byte);
                new_line(out, depth);
            }
            b':' => out.extend_from_slice(b": "),
            _ => out.push(byte),
        }
    }
}

/// Returns the bytes of `value` as an element of an array that sits
/// `depth` levels deep in a document in `layout`, without the separators
/// around it.
pub(super) fn element_len(
    value: &(impl Serialize + ?Sized),
    layout: JsonLayout,
    depth: usize,
) -> io::Result<usize> {
    let compact = serde_json::to_vec(value)?;
    Ok(match layout {
        JsonLayout::Pretty { .. } => {
            let mut indented = Vec::with_capacity(compact.len() * 2);
            indent_at(&compact, depth, &mut indented);
            indented.len()
        }
        JsonLayout::OneLine => compact.len(),
    })
}

/// Returns the bytes that `count` elements, whose own bytes add up to
/// `elements`, add to an empty array whose elements sit `depth` levels deep
/// in a document in `layout`. One line holds the first element alone, and
/// each later element adds a comma. In `JsonLayout::Pretty`, the first
/// element also opens a line at its depth and one before the closing
/// bracket, and each later element opens a line of its own.
pub(super) const fn array_len(
    count: usize,
    elements: usize,
    layout: JsonLayout,
    depth: usize,
) -> usize {
    if count == 0 {
        return 0;
    }
    let (first, later) = match layout {
        JsonLayout::Pretty { .. } => (4 * depth, 2 + 2 * depth),
        JsonLayout::OneLine => (0, 1),
    };
    elements
        .saturating_add(first)
        .saturating_add(later.saturating_mul(count - 1))
}

/// Appends a line feed and two spaces for each level of `depth` to `out`.
fn new_line(out: &mut Vec<u8>, depth: usize) {
    out.push(b'\n');
    out.resize(out.len() + 2 * depth, b' ');
}

#[cfg(test)]
mod tests;
