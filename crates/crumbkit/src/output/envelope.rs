//! The JSON document that every result prints as with `--json`, and the
//! row that `--fields` and the compact columns select.

use std::io::{self, Write};

use serde::Serialize;
use serde::ser::{SerializeMap, Serializer};
use serde_json::Value;

use super::failure::{Entry, FailureDocument};
use crate::Document;

/// Writes the JSON document `{"schema": <id>, "data": <data>}` to `out` on
/// one line, with `"dry_run": true` after `schema` when `dry_run` is set,
/// `"total": <total>` after `data` when `total` is `Some`,
/// then `"summary": <summary>` when `summary` is `Some`, and then
/// `"failures": [...]` when `failures` is `Some`.
pub(super) fn write_document<T, D: Serialize, M: Serialize>(
    out: &mut impl Write,
    document: Document<T>,
    dry_run: bool,
    data: &D,
    total: Option<usize>,
    summary: Option<&M>,
    failures: Option<&[FailureDocument]>,
) -> io::Result<()> {
    let envelope = Envelope {
        document,
        dry_run,
        data,
        total,
        summary,
        failures,
    };
    serde_json::to_writer(&mut *out, &envelope)?;
    out.write_all(b"\n")
}

/// A JSON document: the `schema` id, `dry_run` on a dry run, the result
/// under `data`, a listing's
/// number of matching rows under `total`, a listing's summary under
/// `summary`, and a batch's failures under `failures`.
struct Envelope<'a, T, D, M> {
    document: Document<T>,
    dry_run: bool,
    data: &'a D,
    total: Option<usize>,
    summary: Option<&'a M>,
    failures: Option<&'a [FailureDocument]>,
}

impl<T, D: Serialize, M: Serialize> Serialize for Envelope<'_, T, D, M> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let len = 2
            + usize::from(self.dry_run)
            + usize::from(self.total.is_some())
            + usize::from(self.summary.is_some())
            + usize::from(self.failures.is_some());
        let mut map = serializer.serialize_map(Some(len))?;
        // `format_args!` serializes as a string without building one.
        map.serialize_entry("schema", &format_args!("{}", self.document))?;
        if self.dry_run {
            map.serialize_entry("dry_run", &true)?;
        }
        map.serialize_entry("data", self.data)?;
        if let Some(total) = self.total {
            map.serialize_entry("total", &total)?;
        }
        if let Some(summary) = self.summary {
            map.serialize_entry("summary", summary)?;
        }
        if let Some(failures) = self.failures {
            // Each entry is a failure document without `schema`.
            let entries: Vec<Entry<'_>> = failures.iter().map(Entry).collect();
            map.serialize_entry("failures", &entries)?;
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
