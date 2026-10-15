//! The compile-time refusal of a row type without columns where `finish`
//! and `stream` print each row as its columns.
//!
//! Without `--full`, the table and the JSON document hold only the fields
//! that `Row::COLUMNS` names, so a row type that names none would print
//! its rows as nothing while `total` still counted them. A headed batch is
//! exempt, because its text prints each row's bytes and its JSON holds
//! every field. The compiler cannot tell a headed batch from any other
//! batch, so a batch passes here and crumbkit-test's `check_guide` fails a
//! leaf whose batch is not headed and whose row type has no columns.

use crate::{Row, Rows, Shape};

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
            !R::COLUMNS.is_empty(),
            "a row type that a stream prints needs at least one name in Row::COLUMNS, because each line without --full prints only those fields"
        );
    }
}
