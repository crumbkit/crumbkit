//! The columns that `finish` and `stream` print of each row, and the
//! compile-time refusal of a row type without columns where they print
//! each row as its columns.
//!
//! Without `--full`, the table and the JSON document hold only the fields
//! that `Row::COLUMNS` names, so a row type that names none would print
//! its rows as nothing while `total` still counted them. A headed batch is
//! exempt, because its text prints each row's bytes and its JSON holds
//! every field. The compiler cannot tell a headed batch from any other
//! batch, so a batch passes here and crumbkit-test's `check_guide` fails a
//! leaf whose batch is not headed and whose row type has no columns. A row
//! type that sets `Row::WHOLE` is also exempt, because its JSON holds every
//! field and its text prints each row as one line of JSON when it has no
//! columns (ADR-0065).

use std::io::{self, Write};

use serde_json::Value;

use super::{Output, layout};
use crate::{Document, KeyOrder, Row, Rows, Shape};

/// Fails to compile when `T` is a single row, a `Vec` or a `Listing` whose
/// row type's [`Row::COLUMNS`] is empty. `finish_to` calls it, so a tool
/// that passes such a result to `finish` does not build:
///
/// ```compile_fail
/// #[derive(serde::Serialize, schemars::JsonSchema)]
/// struct Entry { position: u32 }
/// impl crumbkit::Row for Entry { const COLUMNS: &'static [&'static str] = &[]; }
/// use crumbkit::{Document, Error, Listing, Output, Spelling};
///
/// const ENTRIES: Document<Listing<Entry>> =
///     Document::new("reading", "entries", 1, "The entries of a reading list.");
///
/// let result: Result<Listing<Entry>, Error> = Ok(Listing::new(vec![Entry { position: 1 }], 1));
/// crumbkit::finish(&mut Output::default(), Spelling::COLONS, ENTRIES, result);
/// ```
pub(super) const fn require<T: Rows>() {
    const {
        assert!(
            !<T::Row as Row>::COLUMNS.is_empty()
                || <T::Row as Row>::WHOLE
                || !matches!(T::SHAPE, Shape::Row | Shape::List | Shape::Listing),
            "a row type that finish prints needs at least one name in Row::COLUMNS, because the table and the JSON without --full print only those fields"
        );
    }
}

/// Fails to compile when the row type `R` of a stream has an empty
/// [`Row::COLUMNS`]. `stream` and `stream_to` call it, so a tool that
/// opens such a stream does not build:
///
/// ```compile_fail
/// #[derive(serde::Serialize, schemars::JsonSchema)]
/// struct Entry { position: u32 }
/// impl crumbkit::Row for Entry { const COLUMNS: &'static [&'static str] = &[]; }
/// use crumbkit::{Document, Output, Spelling, Stream};
///
/// const ENTRIES_STREAM: Document<Stream<Entry>> =
///     Document::new("reading", "entries-stream", 1, "Each entry of a reading list.");
///
/// let _ = crumbkit::stream(&mut Output::default(), Spelling::COLONS, ENTRIES_STREAM);
/// ```
pub(super) const fn require_for_stream<R: Row>() {
    const {
        assert!(
            !R::COLUMNS.is_empty() || R::WHOLE,
            "a row type that a stream prints needs at least one name in Row::COLUMNS, because each line without --full prints only those fields"
        );
    }
}

/// Returns the columns that the table and the compact JSON show, or an
/// empty list, which does not allocate, when the result prints whole with
/// every field or is a prose result. Unwrapped or headed JSON, and the
/// JSON of a row type that prints whole, hold every field unless
/// `--fields` names some (ADR-0062, ADR-0065).
pub(super) fn columns<'a, T: Rows>(
    output: &'a Output,
    document: Document<T>,
    value: &T,
) -> Vec<&'a str> {
    let unwrapped =
        output.json && (!document.has_envelope() || document.is_headed() || T::Row::WHOLE);
    let selects = if unwrapped {
        !output.fields.is_empty()
    } else {
        !output.full
    };
    if !selects || value.prose().is_some() {
        Vec::new()
    } else if output.fields.is_empty() {
        let long = if output.long {
            T::Row::LONG_COLUMNS
        } else {
            &[]
        };
        T::Row::COLUMNS.iter().chain(long).copied().collect()
    } else {
        output.fields.iter().map(String::as_str).collect()
    }
}

/// Returns each row of `value` serialized once, so its fields can be looked
/// up by their serialized names, or an empty list when there are no
/// `columns` to look up.
pub(super) fn values<T: Rows>(columns: &[&str], value: &T) -> io::Result<Vec<Value>> {
    if columns.is_empty() {
        return Ok(Vec::new());
    }
    let values = value
        .rows()
        .iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<Value>, _>>()?;
    Ok(values)
}

/// Returns whether the text of rows of type `R` prints each row as one line
/// of compact JSON in place of a table, because `R` sets `Row::WHOLE` and
/// names no columns (ADR-0065).
pub(super) const fn json_lines<R: Row>() -> bool {
    R::WHOLE && R::COLUMNS.is_empty() && R::LONG_COLUMNS.is_empty()
}

/// Writes `row` to `out` as one line of compact JSON, with the keys of each
/// object in `keys` order.
pub(super) fn write_json_line<R: Row>(
    out: &mut impl Write,
    row: &R,
    keys: Option<KeyOrder>,
) -> io::Result<()> {
    layout::serialize(&mut *out, row, keys)?;
    out.write_all(b"\n")
}
