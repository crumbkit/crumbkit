//! The layout of a whole JSON document: one line by default, or indented
//! when the tool's guide sets [`JsonLayout::Pretty`] (ADR-0052). The keys
//! of each object keep the order that the value serializes them in, or
//! print sorted when the guide sets [`KeyOrder::Sorted`] (ADR-0067).

use std::io::{self, Write};

use serde::Serialize;

/// The layout of a whole JSON document that a tool sets with
/// [`Guide::json_layout`](crate::help::Guide::json_layout).
///
/// Without a layout, every JSON document prints on one line, whether or not
/// standard output is a terminal. A stream prints one document per line
/// with any layout, because JSON Lines needs it.
///
/// The enum is `#[non_exhaustive]`, so crumbkit can add a layout without
/// breaking a tool that matches on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum JsonLayout {
    /// Each field and each item on a line of its own, indented by two
    /// spaces for each level, as `serde_json::to_writer_pretty` writes it.
    Pretty,
}

/// The order of the keys of each object in the JSON that a tool prints,
/// which the tool sets with
/// [`Guide::json_keys`](crate::help::Guide::json_keys).
///
/// Without an order, each object prints its keys in the order that its
/// value serializes them: crumbkit's envelope prints the type key first,
/// and a row prints its fields in their declaration order.
///
/// The enum is `#[non_exhaustive]`, so crumbkit can add an order without
/// breaking a tool that matches on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeyOrder {
    /// The keys of every object, at every depth, in the order of Rust's
    /// `str` comparison, which compares the keys' Unicode code points.
    Sorted,
}

/// How a run writes JSON on standard output: the layout and the key order
/// that the guide sets. Its default is one line with the keys in the
/// order that each value serializes them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct JsonStyle {
    /// The layout of a whole document, or `None` for one line.
    pub(crate) layout: Option<JsonLayout>,
    /// The order of the keys of each object, or `None` for the order that
    /// each value serializes them.
    pub(crate) keys: Option<KeyOrder>,
}

impl JsonStyle {
    /// Returns the same style on one line, as a stream and the line of a
    /// headed answer need it.
    pub(crate) const fn one_line(self) -> Self {
        Self {
            layout: None,
            keys: self.keys,
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
        Some(JsonLayout::Pretty) => {
            let mut compact = Vec::new();
            serialize(&mut compact, value, style.keys)?;
            write_indented(out, &compact)
        }
        None => {
            serialize(out, value, style.keys)?;
            out.write_all(b"\n")
        }
    }
}

/// Writes `value` to `out` on one line, with the keys of each object in
/// `keys` order.
///
/// With [`KeyOrder::Sorted`], it serializes `value` into a
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
    keys: Option<KeyOrder>,
) -> io::Result<()> {
    match keys {
        Some(KeyOrder::Sorted) => {
            let mut sorted = serde_json::to_value(value)?;
            sorted.sort_all_objects();
            Ok(serde_json::to_writer(out, &sorted)?)
        }
        None => Ok(serde_json::to_writer(out, value)?),
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
    let mut depth = 0_usize;
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

/// Appends a line feed and two spaces for each level of `depth` to `out`.
fn new_line(out: &mut Vec<u8>, depth: usize) {
    out.push(b'\n');
    out.resize(out.len() + 2 * depth, b' ');
}

#[cfg(test)]
mod tests;
