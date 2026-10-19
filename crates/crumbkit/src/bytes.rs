//! The result of a command whose answer is a file's bytes, and the JSON form
//! of bytes in a row.

use std::borrow::Cow;
use std::fmt;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::Serialize;
use serde::ser::{SerializeMap, Serializer};

use crate::{NoRow, Rows, Shape};

#[cfg(test)]
mod tests;

/// The result of a command whose answer is bytes, such as a file's
/// contents, the way `cat` prints a file. It is also the type of a row
/// field that holds bytes, so a [`Batch`](crate::Batch) can return several
/// files.
///
/// `crumbkit::finish` writes the bytes to standard output unchanged, with
/// no table, no escaping and no added line feed. This is the one result
/// whose text crumbkit does not escape, so bytes that hold a terminal's
/// escape sequences reach a terminal as they would through `cat`. With
/// `--json`, the document holds `{"text": "..."}` under `data` when the
/// bytes are valid UTF-8, and `{"base64": "..."}` in standard base64 with
/// padding (RFC 4648) otherwise. A row field of this type serializes in the
/// same form. A table shows such a field as compact JSON, so a row type
/// usually keeps it out of its columns, and a caller reads it with `--json
/// --full` or `--fields`.
///
/// A row field marked `#[serde(flatten)]` puts `text` or `base64` at the top
/// level of the row, and the row's JSON Schema then lists both as
/// properties, so `--fields text` names them. An `Option<Bytes>` field
/// flattens in the same way and adds nothing when it is `None`. A row type
/// that flattens bytes has a doc comment of its own, because without one
/// its schema takes the description of the bytes. A batch whose rows hold
/// bytes can print them in text as `head` prints files
/// ([`Headed`](crate::Headed)).
///
/// `--fields`, `--long` and `--full` are refused for a bytes result, with
/// the exit code of a refused command line, because it has no columns. The result is never cut short:
/// when it does not fit in the output budget, it is refused, unless its
/// document always prints its first answer.
///
/// The bytes are held in memory. A handler that reads a file can read at
/// most one byte more than `Output::budget` with `Read::take`, and return a
/// failure of its own when that byte arrives.
///
/// ```
/// use crumbkit::Bytes;
///
/// let bytes = Bytes::from("North and South\n".to_owned());
///
/// assert_eq!(bytes.as_slice(), b"North and South\n");
/// ```
//
// RUST-10: the result holds every byte, because `--json` must see them all
// to choose between `text` and `base64`, and the output budget must measure
// them before anything prints. A file larger than memory needs a reader
// that can seek back to its start, which replaces the `Vec` once a tool
// prints such files.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bytes {
    /// The bytes, in the order they print.
    bytes: Vec<u8>,
}

impl Bytes {
    /// Returns a bytes result that holds `bytes`.
    #[must_use]
    pub const fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    /// Returns the bytes.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.bytes
    }
}

impl From<Vec<u8>> for Bytes {
    fn from(bytes: Vec<u8>) -> Self {
        Self::new(bytes)
    }
}

impl From<String> for Bytes {
    fn from(text: String) -> Self {
        Self::new(text.into_bytes())
    }
}

/// Serializes the bytes as `{"text": "..."}` when they are valid UTF-8, and
/// as `{"base64": "..."}` otherwise.
impl Serialize for Bytes {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(1))?;
        match std::str::from_utf8(&self.bytes) {
            Ok(text) => map.serialize_entry("text", text)?,
            // `format_args!` serializes as a string without building one.
            Err(_) => map.serialize_entry("base64", &format_args!("{}", Base64(&self.bytes)))?,
        }
        map.end()
    }
}

/// Describes the bytes as an object with the properties `text` and
/// `base64`, of which it holds exactly one, under the definition name
/// `Bytes`. The two properties sit at the top level of the schema, so a row
/// that flattens the bytes lists them among its own properties.
impl JsonSchema for Bytes {
    fn schema_name() -> Cow<'static, str> {
        "Bytes".into()
    }

    fn schema_id() -> Cow<'static, str> {
        "crumbkit::Bytes".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "description": "Bytes, such as a file's contents: text when they are valid UTF-8, and base64 otherwise.",
            "type": "object",
            "properties": {
                "text": {
                    "description": "The bytes, which are valid UTF-8, as text.",
                    "type": "string",
                },
                "base64": {
                    "description": "The bytes, which are not valid UTF-8, in standard base64 with padding (RFC 4648).",
                    "type": "string",
                    "contentEncoding": "base64",
                },
            },
            "oneOf": [
                { "required": ["text"] },
                { "required": ["base64"] },
            ],
            "additionalProperties": false,
        })
    }
}

impl Rows for Bytes {
    type Row = NoRow;
    const SHAPE: Shape = Shape::Bytes;
    type Summary = crate::NoSummary;

    fn rows(&self) -> &[NoRow] {
        &[]
    }

    fn bytes(&self) -> Option<&[u8]> {
        Some(&self.bytes)
    }
}

/// The standard base64 alphabet of RFC 4648.
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// The number of characters of base64 text that [`Base64`] writes at a
/// time. It is a multiple of 4, so each piece encodes whole groups of three
/// bytes.
const PIECE: usize = 1024;

/// Bytes whose `Display` writes them in standard base64 with padding, one
/// piece of [`PIECE`] characters at a time, so serializing them builds no
/// string.
//
// RUST-10: this encodes with the standard alphabet and padding, which is all
// that `Bytes` prints. It does not decode, and it has no URL-safe alphabet
// and no unpadded form. The base64 crate replaces it once crumbkit needs
// any of those.
pub(crate) struct Base64<'a>(pub(crate) &'a [u8]);

impl fmt::Display for Base64<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut piece = [0_u8; PIECE];
        for bytes in self.0.chunks(PIECE / 4 * 3) {
            let mut written = 0;
            for group in bytes.chunks(3) {
                let first = group.first().copied().unwrap_or(0);
                let second = group.get(1).copied().unwrap_or(0);
                let third = group.get(2).copied().unwrap_or(0);
                let joined = u32::from(first) << 16 | u32::from(second) << 8 | u32::from(third);
                let third_character = if group.len() > 1 {
                    sextet(joined >> 6)
                } else {
                    b'='
                };
                let fourth_character = if group.len() > 2 {
                    sextet(joined)
                } else {
                    b'='
                };
                let characters = [
                    sextet(joined >> 18),
                    sextet(joined >> 12),
                    third_character,
                    fourth_character,
                ];
                if let Some(slot) = piece.get_mut(written..written + 4) {
                    slot.copy_from_slice(&characters);
                }
                written += 4;
            }
            let text = piece.get(..written).unwrap_or_default();
            // Every byte of the piece is a character of the alphabet or `=`,
            // so it is ASCII and the conversion does not fail.
            f.write_str(std::str::from_utf8(text).map_err(|_| fmt::Error)?)?;
        }
        Ok(())
    }
}

/// Returns the alphabet's character for the low six bits of `bits`.
fn sextet(bits: u32) -> u8 {
    let index = usize::try_from(bits & 0x3f).unwrap_or(0);
    // The index is below 64, so the lookup always finds a character.
    ALPHABET.get(index).copied().unwrap_or(b'=')
}
