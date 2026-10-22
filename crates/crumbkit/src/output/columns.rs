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
//! type that sets `Row::WHOLE` and declares `Row::LINE` is also exempt,
//! because its JSON holds every field and its text prints each row's own
//! line (ADR-0065, ADR-0072).

use std::io;

use serde_json::Value;

use crate::view::View;
use crate::{Document, Row, Rows, Shape};

/// Fails to compile when `T` is a single row, a `Vec` or a `Listing` whose
/// row type's [`Row::COLUMNS`] is empty. `finish_to` calls it, so a tool
/// that passes such a result to `finish` does not build:
///
/// ```compile_fail
/// #[derive(serde::Serialize, schemars::JsonSchema)]
/// struct Entry { position: u32 }
/// impl crumbkit::Row for Entry { const COLUMNS: &'static [&'static str] = &[]; }
/// use crumbkit::{Document, Error, Listing, Output};
///
/// const ENTRIES: Document<Listing<Entry>> =
///     Document::new("reading", "entries", 1, "The entries of a reading list.");
///
/// let result: Result<Listing<Entry>, Error> = Ok(Listing::new(vec![Entry { position: 1 }], 1));
/// # let output = Output::new(crumbkit::ViewFlags::new(), crumbkit::DryRun::default(), clap::Command::new("reading"), &crumbkit::help::Guide::new(), ["reading"]).unwrap();
/// crumbkit::finish(output, ENTRIES, result);
/// ```
///
/// A row type that sets [`Row::WHOLE`] may name no column only when it
/// also declares [`Row::LINE`], so its text prints each row's own line
/// (ADR-0072). Without a `LINE`, it does not build either:
///
/// ```compile_fail
/// #[derive(serde::Serialize, schemars::JsonSchema)]
/// struct Answer(u32, u32);
/// impl crumbkit::Row for Answer {
///     const COLUMNS: &'static [&'static str] = &[];
///     const WHOLE: bool = true;
/// }
/// use crumbkit::{Document, Error, Output};
///
/// const ANSWERS: Document<Vec<Answer>> =
///     Document::new("reading", "answers", 1, "The answers of a query.");
///
/// let result: Result<Vec<Answer>, Error> = Ok(vec![Answer(1, 2)]);
/// # let output = Output::new(crumbkit::ViewFlags::new(), crumbkit::DryRun::default(), clap::Command::new("reading"), &crumbkit::help::Guide::new(), ["reading"]).unwrap();
/// crumbkit::finish(output, ANSWERS, result);
/// ```
pub(super) const fn require<T: Rows>() {
    const {
        assert!(
            !<T::Row as Row>::COLUMNS.is_empty()
                || (<T::Row as Row>::WHOLE && <T::Row as Row>::LINE.is_some())
                || !matches!(T::SHAPE, Shape::Row | Shape::List | Shape::Listing),
            "a row type that finish prints needs at least one name in Row::COLUMNS, because the table and the JSON without --full print only those fields, or needs Row::WHOLE with a Row::LINE"
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
/// use crumbkit::{Document, Output, Stream};
///
/// const ENTRIES_STREAM: Document<Stream<Entry>> =
///     Document::new("reading", "entries-stream", 1, "Each entry of a reading list.");
///
/// # let output = Output::new(crumbkit::ViewFlags::new(), crumbkit::DryRun::default(), clap::Command::new("reading"), &crumbkit::help::Guide::new(), ["reading"]).unwrap();
/// let _ = crumbkit::stream(output, ENTRIES_STREAM);
/// ```
///
/// A stream of a row type that sets [`Row::WHOLE`], names no column and
/// declares no [`Row::LINE`] does not build either (ADR-0072):
///
/// ```compile_fail
/// #[derive(serde::Serialize, schemars::JsonSchema)]
/// struct Answer(u32, u32);
/// impl crumbkit::Row for Answer {
///     const COLUMNS: &'static [&'static str] = &[];
///     const WHOLE: bool = true;
/// }
/// use crumbkit::{Document, Output, Stream};
///
/// const ANSWERS: Document<Stream<Answer>> =
///     Document::new("reading", "answers-stream", 1, "Each answer of a query.");
///
/// # let output = Output::new(crumbkit::ViewFlags::new(), crumbkit::DryRun::default(), clap::Command::new("reading"), &crumbkit::help::Guide::new(), ["reading"]).unwrap();
/// let _ = crumbkit::stream(output, ANSWERS);
/// ```
pub(super) const fn require_for_stream<R: Row>() {
    const {
        assert!(
            !R::COLUMNS.is_empty() || (R::WHOLE && R::LINE.is_some()),
            "a row type that a stream prints needs at least one name in Row::COLUMNS, because each line without --full prints only those fields, or needs Row::WHOLE with a Row::LINE"
        );
    }
}

/// Returns the columns that the table and the compact JSON show, or an
/// empty list, which does not allocate, when the result prints whole with
/// every field, is a prose result, which `prose` marks, or prints each
/// row's own text line. Unwrapped or headed JSON, and the JSON of a row type
/// that prints whole, hold every field unless `--fields` names some
/// (ADR-0062, ADR-0065). The JSON without `--fields` holds the field that
/// [`Row::CHILDREN`] names after the columns, and the text table does not
/// (ADR-0120).
pub(super) fn columns<T: Rows>(view: &View, document: Document<T>, prose: bool) -> Vec<&str> {
    if super::lines::chosen::<T::Row>(view) {
        return Vec::new();
    }
    let unwrapped = view.is_json()
        && (!document.get_envelope().wraps() || document.is_headed() || T::Row::WHOLE);
    let selects = if unwrapped {
        view.names().is_some()
    } else {
        !view.is_full()
    };
    if !selects || prose {
        Vec::new()
    } else if let Some(names) = view.names() {
        names.iter().map(String::as_str).collect()
    } else {
        let long = if view.is_long() {
            T::Row::LONG_COLUMNS
        } else {
            &[]
        };
        let mut names: Vec<&str> = T::Row::COLUMNS.iter().chain(long).copied().collect();
        if view.is_json() {
            names.extend(super::children::json_field::<T::Row>(&names));
        }
        names
    }
}

/// Returns each of `rows` serialized once, so its fields can be looked up
/// by their serialized names, or an empty list when there are no `columns`
/// to look up.
pub(super) fn values<'a, R: Row + 'a>(
    columns: &[&str],
    rows: impl Iterator<Item = &'a R>,
) -> io::Result<Vec<Value>> {
    if columns.is_empty() {
        return Ok(Vec::new());
    }
    let values = rows
        .map(serde_json::to_value)
        .collect::<Result<Vec<Value>, _>>()?;
    Ok(values)
}
