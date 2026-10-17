//! The layout of the JSON that a tool prints: one line by default, or
//! indented when the tool's guide sets [`JsonLayout::Pretty`] (ADR-0052).
//! The keys of each object keep the order that the value serializes them
//! in.

use std::io::{self, Write};

use serde::Serialize;

/// The layout of a whole JSON document, which a tool sets with
/// [`Guide::json_layout`](crate::help::Guide::json_layout) (ADR-0052).
///
/// A stream prints one document per line with any layout, because JSON
/// Lines needs it.
///
/// The enum is `#[non_exhaustive]`, so crumbkit can add a layout without
/// breaking a tool that matches on it.
///
/// ```
/// use crumbkit::JsonLayout;
/// use crumbkit::help::Guide;
///
/// const PRETTY: JsonLayout = JsonLayout::Pretty {
///     reason: "Our scripts read indented JSON.",
/// };
/// const GUIDE: Guide = Guide::new().json_layout(PRETTY);
/// assert_eq!(GUIDE.get_json_layout(), PRETTY);
/// assert_eq!(Guide::new().get_json_layout(), JsonLayout::OneLine);
/// ```
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

/// Writes `value` to `out` as JSON in `layout`, and ends it with a line
/// feed.
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
    layout: JsonLayout,
) -> io::Result<()> {
    match layout {
        JsonLayout::Pretty { .. } => {
            let mut compact = Vec::new();
            serialize(&mut compact, value)?;
            write_indented(out, &compact)
        }
        JsonLayout::OneLine => {
            serialize(out, value)?;
            out.write_all(b"\n")
        }
    }
}

/// Writes `value` to `out` on one line.
pub(super) fn serialize(out: &mut dyn Write, value: &(impl Serialize + ?Sized)) -> io::Result<()> {
    Ok(serde_json::to_writer(out, value)?)
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
