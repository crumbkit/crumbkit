//! How a stream prints the JSON of each item of a document without
//! crumbkit's envelope, one item per line (ADR-0016, ADR-0041).

use std::io::{self, Write};

use serde_json::Value;

use super::envelope::Selected;
use super::layout;
use crate::{KeyOrder, Rows, Shape};

/// Writes `value` to `out` as JSON on one line, without `schema`, `total`,
/// `summary` or `failures`, then flushes `out`.
///
/// With no `fields`, it writes the value as its type serializes it, with
/// every field, so a command prints an existing document unchanged: a row
/// as an object, a prose result as its value, and every other result as an
/// array of its rows. With `fields`, it writes only those fields of each
/// row, in that order, as an object for a single row and as an array
/// otherwise. The keys of each object print in `keys` order. A
/// serialization error becomes an `io::Error` of kind `InvalidData`, as in
/// the envelope.
pub(super) fn write_json<T: Rows>(
    out: &mut impl Write,
    fields: &[String],
    value: &T,
    keys: Option<KeyOrder>,
) -> io::Result<()> {
    if fields.is_empty() {
        layout::serialize(&mut *out, value, keys)?;
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
            [single] if T::SHAPE == Shape::Row => layout::serialize(&mut *out, single, keys)?,
            list => layout::serialize(&mut *out, list, keys)?,
        }
    }
    out.write_all(b"\n")?;
    out.flush()
}
