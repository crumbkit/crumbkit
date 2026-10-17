//! The JSON document that every result prints as with `--json`, and the
//! row that `--fields` and the compact columns select.

use std::io::{self, Write};

use serde::Serialize;
use serde::ser::{SerializeMap, Serializer};
use serde_json::Value;

use super::budget::Cut;
use super::failure::{Entry, PreparedFailure};
use super::layout;
use super::layout::JsonLayout;
use crate::Document;

/// The failures of a batch or a prose result: the tool's own, in the order
/// of their keys, and then the failure that names the answers that the
/// output budget left out, when there is one.
#[derive(Clone, Copy)]
pub(super) struct Failures<'a> {
    /// The failures that the tool returned.
    pub(super) listed: &'a [PreparedFailure<'a>],
    /// The failure that names the answers that did not fit in the output
    /// budget.
    pub(super) over: Option<&'a PreparedFailure<'a>>,
}

impl<'a> Failures<'a> {
    /// Returns the tool's failures `listed`, without a failure of the output
    /// budget.
    pub(super) const fn of(listed: &'a [PreparedFailure<'a>]) -> Self {
        Self { listed, over: None }
    }

    /// Returns every failure in the order it prints: the tool's failures,
    /// then the failure that names the answers left out.
    pub(super) fn iter(self) -> impl Iterator<Item = &'a PreparedFailure<'a>> {
        self.listed.iter().chain(self.over)
    }
}

/// The parts of a JSON document beside `schema` and `data`, each of which
/// the document leaves out when it is `None`.
pub(super) struct Extras<'a, M> {
    /// A listing's number of matching rows, when the tool counted them.
    pub(super) total: Option<usize>,
    /// Whether a listing's document leaves out rows.
    pub(super) truncated: Option<bool>,
    /// A listing's summary.
    pub(super) summary: Option<&'a M>,
    /// The mark of a result that the output budget cut short.
    pub(super) cut: Option<&'a Cut>,
    /// A batch's or a prose result's failures.
    pub(super) failures: Option<Failures<'a>>,
}

impl Extras<'_, ()> {
    /// Returns the extras of a document that has none.
    pub(super) const fn none() -> Self {
        Self {
            total: None,
            truncated: None,
            summary: None,
            cut: None,
            failures: None,
        }
    }
}

/// Writes the JSON document `{"schema": <id>, "data": <data>}` to `out` in
/// `layout`, with `"dry_run": true`
/// after `schema` when `dry_run` is set, `"total": <total>` after `data`
/// when the extras hold a total, then `"truncated": <bool>`,
/// `"summary": <summary>`, `"cut": {...}`
/// and `"failures": [...]` when the extras hold each of them.
pub(super) fn write_document<T, D: Serialize, M: Serialize>(
    out: &mut impl Write,
    document: Document<T>,
    dry_run: bool,
    layout: JsonLayout,
    data: &D,
    extras: &Extras<'_, M>,
) -> io::Result<()> {
    let wrapped = Wrapped {
        document,
        dry_run,
        data,
        extras,
    };
    layout::write(out, &wrapped, layout)
}

/// A JSON document: `schema`, `dry_run` on a dry run, the result under
/// `data`, and the extras.
struct Wrapped<'a, T, D, M> {
    document: Document<T>,
    dry_run: bool,
    data: &'a D,
    extras: &'a Extras<'a, M>,
}

impl<T, D: Serialize, M: Serialize> Serialize for Wrapped<'_, T, D, M> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let extras = self.extras;
        let len = 2
            + usize::from(self.dry_run)
            + usize::from(extras.total.is_some())
            + usize::from(extras.truncated.is_some())
            + usize::from(extras.summary.is_some())
            + usize::from(extras.cut.is_some())
            + usize::from(extras.failures.is_some());
        let document = self.document;
        let mut map = serializer.serialize_map(Some(len))?;
        // `format_args!` serializes as a string without building one.
        map.serialize_entry("schema", &format_args!("{document}"))?;
        if self.dry_run {
            map.serialize_entry("dry_run", &true)?;
        }
        map.serialize_entry("data", self.data)?;
        if let Some(total) = extras.total {
            map.serialize_entry("total", &total)?;
        }
        if let Some(truncated) = extras.truncated {
            map.serialize_entry("truncated", &truncated)?;
        }
        if let Some(summary) = extras.summary {
            map.serialize_entry("summary", summary)?;
        }
        if let Some(cut) = extras.cut {
            map.serialize_entry("cut", cut)?;
        }
        if let Some(failures) = extras.failures {
            map.serialize_entry("failures", &Entries(failures))?;
        }
        map.end()
    }
}

/// One serialized row, limited to `columns` and written in their order.
pub(super) struct Selected<'a> {
    pub(super) row: &'a Value,
    pub(super) columns: &'a [&'a str],
}

impl Serialize for Selected<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.columns.len()))?;
        for &name in self.columns {
            // A name that is not a field of the row writes `null`, as the
            // table writes `-`.
            map.serialize_entry(name, self.row.get(name).unwrap_or(&Value::Null))?;
        }
        map.end()
    }
}

/// The entries of `failures`, written one by one so that nothing is
/// collected first: each a failure document without `schema`.
struct Entries<'a>(Failures<'a>);

impl Serialize for Entries<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.0.iter().map(Entry))
    }
}

/// Serialized rows, each limited to `columns` and written in their order,
/// as one array.
pub(super) struct SelectedRows<'a> {
    pub(super) rows: &'a [Value],
    pub(super) columns: &'a [&'a str],
}

impl Serialize for SelectedRows<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.rows.iter().map(|row| Selected {
            row,
            columns: self.columns,
        }))
    }
}
