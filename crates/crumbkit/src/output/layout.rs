//! The layout of a whole JSON document: one line by default, or indented
//! when the tool's guide sets [`JsonLayout::Pretty`] (ADR-0052).

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

/// Writes `value` to `out` as JSON in `layout`, or on one line when
/// `layout` is `None`, and ends it with a line feed.
///
/// A serialization error becomes an `io::Error` of kind `InvalidData`, so it
/// is reported as a failed write.
pub(super) fn write(
    out: &mut impl Write,
    value: &(impl Serialize + ?Sized),
    layout: Option<JsonLayout>,
) -> io::Result<()> {
    match layout {
        Some(JsonLayout::Pretty) => serde_json::to_writer_pretty(&mut *out, value)?,
        None => serde_json::to_writer(&mut *out, value)?,
    }
    out.write_all(b"\n")
}
