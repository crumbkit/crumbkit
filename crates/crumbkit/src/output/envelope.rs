//! The JSON document that every result prints as with `--json`, and the
//! row that `--fields` and the compact columns select.

use std::io::{self, Write};

use serde::Serialize;
use serde::ser::{Error as _, SerializeMap, Serializer};
use serde_json::Value;

use super::budget::Cut;
use super::failure::{Entry, PreparedFailure};
use super::layout;
use super::layout::JsonStyle;
use crate::Document;
use crate::form::FailureForm;

/// The failures of a batch or a prose result: the tool's own and the
/// failure of each key that the output budget left out, in the order of
/// their keys, and then the failure that names the answers that the output
/// budget left out, when there is one.
#[derive(Clone, Copy)]
pub(super) struct Failures<'a> {
    /// The failures that the tool returned.
    pub(super) listed: &'a [PreparedFailure<'a>],
    /// The position of each failure of `listed` among a batch's answers. A
    /// failure without a position sorts after every other.
    pub(super) indexes: &'a [usize],
    /// The failure of each answer that the output budget left out, with its
    /// position, in order, when the document answers each key left out
    /// (ADR-0068).
    pub(super) each: &'a [(usize, PreparedFailure<'a>)],
    /// The failure that names the answers that did not fit in the output
    /// budget.
    pub(super) over: Option<&'a PreparedFailure<'a>>,
    /// The guide's failure form, which writes each entry, or `None` for
    /// crumbkit's entries.
    pub(super) form: Option<&'static dyn FailureForm>,
}

impl<'a> Failures<'a> {
    /// Returns the tool's failures `listed` at the positions `indexes`,
    /// which the guide's `form` writes, without a failure of the output
    /// budget.
    pub(super) const fn of(
        listed: &'a [PreparedFailure<'a>],
        indexes: &'a [usize],
        form: Option<&'static dyn FailureForm>,
    ) -> Self {
        Self {
            listed,
            indexes,
            each: &[],
            over: None,
            form,
        }
    }

    /// Returns every failure in the order it prints.
    pub(super) fn iter(self) -> impl Iterator<Item = &'a PreparedFailure<'a>> {
        self.positioned().map(|(_, failure)| failure)
    }

    /// Returns every failure in the order it prints with its position among
    /// the batch's answers: the tool's failures and the failure of each key
    /// left out merged by position, then the failure that names the answers
    /// left out at `usize::MAX`.
    pub(super) fn positioned(self) -> impl Iterator<Item = (usize, &'a PreparedFailure<'a>)> {
        let indexes = self.indexes;
        let mut listed = self
            .listed
            .iter()
            .enumerate()
            .map(move |(number, failure)| {
                let position = indexes.get(number).copied().unwrap_or(usize::MAX);
                (position, failure)
            })
            .peekable();
        let mut each = self
            .each
            .iter()
            .map(|(position, failure)| (*position, failure))
            .peekable();
        let merged = std::iter::from_fn(move || match (listed.peek(), each.peek()) {
            (Some((tool, _)), Some((budget, _))) if budget < tool => each.next(),
            (Some(_), _) => listed.next(),
            (None, _) => each.next(),
        });
        merged.chain(self.over.map(|over| (usize::MAX, over)))
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

/// The fields that crumbkit writes beside the result in an envelope, which
/// no field of a summary at the top of a document can take.
/// `Document::with_keys` refuses the same names as keys.
pub(crate) const WRITTEN: [&str; 6] = [
    "dry_run",
    "total",
    "truncated",
    "summary",
    "cut",
    "failures",
];

/// Writes the JSON document `{"schema": <id>, "data": <data>}` to `out` in
/// `layout`, or on one line when `layout` is `None`, with `"dry_run": true`
/// after `schema` when `dry_run` is set, `"total": <total>` after `data`
/// when the extras hold a total, then `"truncated": <bool>`,
/// `"summary": <summary>`, `"cut": {...}`
/// and `"failures": [...]` when the extras hold each of them.
///
/// A document with its own keys writes its type key in place of `schema`,
/// holding its type value or its id, and its data key in place of `data`.
/// A document whose summary prints at the top writes each field of the
/// summary in place of `summary` (ADR-0059).
pub(super) fn write_document<T, D: Serialize, M: Serialize>(
    out: &mut impl Write,
    document: Document<T>,
    dry_run: bool,
    style: JsonStyle,
    data: &D,
    extras: &Extras<'_, M>,
) -> io::Result<()> {
    let envelope = Envelope {
        document,
        dry_run,
        data,
        extras,
    };
    layout::write(out, &envelope, style)
}

/// A JSON document: the type key, `dry_run` on a dry run, the result under
/// the data key, and the extras.
struct Envelope<'a, T, D, M> {
    document: Document<T>,
    dry_run: bool,
    data: &'a D,
    extras: &'a Extras<'a, M>,
}

impl<T, D: Serialize, M: Serialize> Serialize for Envelope<'_, T, D, M> {
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
        let at_top = document.prints_summary_at_top();
        // The fields of a summary at the top are counted only once they are
        // serialized, so the map's length is then unknown.
        let len = (!at_top).then_some(len);
        let mut map = serializer.serialize_map(len)?;
        match document.type_value() {
            Some(value) => map.serialize_entry(document.type_key(), value)?,
            // `format_args!` serializes as a string without building one.
            None => map.serialize_entry(document.type_key(), &format_args!("{document}"))?,
        }
        if self.dry_run {
            map.serialize_entry("dry_run", &true)?;
        }
        map.serialize_entry(document.data_key(), self.data)?;
        if let Some(total) = extras.total {
            map.serialize_entry("total", &total)?;
        }
        if let Some(truncated) = extras.truncated {
            map.serialize_entry("truncated", &truncated)?;
        }
        match extras.summary {
            Some(summary) if at_top => {
                for (name, value) in top_fields(document, summary).map_err(S::Error::custom)? {
                    map.serialize_entry(&name, &value)?;
                }
            }
            Some(summary) => map.serialize_entry("summary", summary)?,
            None => {}
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

/// Returns the fields of `summary`, which prints at the top of `document`,
/// or the reason it cannot: the summary is not an object, or one of its
/// fields takes the document's type key, its data key or a field that
/// crumbkit writes (ADR-0059).
fn top_fields<T, M: Serialize>(
    document: Document<T>,
    summary: &M,
) -> Result<serde_json::Map<String, Value>, String> {
    let Value::Object(fields) = serde_json::to_value(summary).map_err(|error| error.to_string())?
    else {
        return Err(format!(
            "the summary of {document} prints at the top of the document, so it must serialize as an object"
        ));
    };
    let taken = |name: &str| {
        name == document.type_key() || name == document.data_key() || WRITTEN.contains(&name)
    };
    if let Some(name) = fields.keys().find(|name| taken(name)) {
        return Err(format!(
            "the summary of {document} prints at the top of the document, and its field `{name}` takes a name that the document already holds"
        ));
    }
    Ok(fields)
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
/// collected first: each the form's entry when the guide has a failure
/// form, and otherwise a failure document without `schema`.
struct Entries<'a>(Failures<'a>);

impl Serialize for Entries<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0.form {
            Some(form) => serializer.collect_seq(self.0.iter().map(|failure| form.entry(failure))),
            None => serializer.collect_seq(self.0.iter().map(Entry)),
        }
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
