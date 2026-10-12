//! How `finish` prints the JSON of a document without crumbkit's envelope
//! (ADR-0016).

use std::io::{self, Write};

use serde_json::Value;

use super::Selected;
use crate::{Rows, Shape};

/// Writes `value` to `out` as JSON on one line, without `schema`, `total`,
/// `summary` or `failures`, then flushes `out`.
///
/// With no `fields`, it writes the value as its type serializes it, with
/// every field, so a command prints an existing document unchanged: a row
/// as an object, a prose result as its value, and every other result as an
/// array of its rows. With `fields`, it writes only those fields of each
/// row, in that order, as an object for a single row and as an array
/// otherwise. A serialization error becomes an `io::Error` of kind
/// `InvalidData`, as in the envelope.
pub(super) fn write_json<T: Rows>(
    out: &mut impl Write,
    fields: &[String],
    value: &T,
) -> io::Result<()> {
    if fields.is_empty() {
        serde_json::to_writer(&mut *out, value)?;
    } else {
        let columns: Vec<&str> = fields.iter().map(String::as_str).collect();
        let rows = value
            .rows()
            .iter()
            .map(serde_json::to_value)
            .collect::<Result<Vec<Value>, _>>()?;
        let selected: Vec<Selected<'_>> = rows
            .iter()
            .map(|row| Selected {
                row,
                columns: &columns,
            })
            .collect();
        match selected.as_slice() {
            [single] if T::SHAPE == Shape::Row => serde_json::to_writer(&mut *out, single)?,
            list => serde_json::to_writer(&mut *out, list)?,
        }
    }
    out.write_all(b"\n")?;
    out.flush()
}
