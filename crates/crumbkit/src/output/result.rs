//! `write_result`, which writes a result that is not a failure in the
//! format that the output flags choose.

use std::io::{self, Write};

use serde_json::Value;

use super::dry_run::DRY_RUN_LINE;
use super::envelope::{Selected, write_document};
use super::failure::PreparedFailure;
use super::{Output, listing, prose, table, unwrapped};
use crate::{Document, Row, Rows, Shape};

/// Writes `value` to `out` in the format that `output` chooses, then flushes
/// `out`. `failures` holds the failures of a batch or a prose result, which
/// the JSON document holds under `failures`, and is `None` for every other
/// result.
///
/// A serialization error becomes an `io::Error` of kind `InvalidData`, so it
/// is reported as a failed write. The text of a dry run ends with
/// [`DRY_RUN_LINE`].
pub(super) fn write_result<T: Rows>(
    out: &mut impl Write,
    output: &mut Output,
    document: Document<T>,
    value: &T,
    failures: Option<&[PreparedFailure<'_>]>,
) -> io::Result<()> {
    write_value(out, output, document, value, failures)?;
    if output.dry_run && !output.json {
        writeln!(out, "{DRY_RUN_LINE}")?;
    }
    out.flush()
}

/// Writes `value` to `out` as [`write_result`] describes, without the line
/// of a dry run.
fn write_value<T: Rows>(
    out: &mut impl Write,
    output: &mut Output,
    document: Document<T>,
    value: &T,
    failures: Option<&[PreparedFailure<'_>]>,
) -> io::Result<()> {
    if output.json && !document.has_envelope() {
        return unwrapped::write_json(out, &output.fields, value);
    }
    if let Some(text) = value.prose() {
        if output.json {
            write_document(
                out,
                document,
                output.dry_run,
                value,
                None,
                None::<&()>,
                failures,
            )?;
        } else {
            prose::write_text(out, text)?;
        }
        return out.flush();
    }
    let total = value.total();
    let summary = value.summary();
    if output.full {
        // Serializing the rows directly keeps every field in its declaration
        // order.
        write_document(
            out,
            document,
            output.dry_run,
            value,
            total,
            summary,
            failures,
        )?;
        return out.flush();
    }

    let columns: Vec<&str> = if output.fields.is_empty() {
        let long = if output.long {
            T::Row::LONG_COLUMNS
        } else {
            &[]
        };
        T::Row::COLUMNS.iter().chain(long).copied().collect()
    } else {
        output.fields.iter().map(String::as_str).collect()
    };
    // Each row is serialized once, so its fields can be looked up by their
    // serialized names.
    let rows = value
        .rows()
        .iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<Value>, _>>()?;

    if output.json {
        let selected: Vec<Selected<'_>> = rows
            .iter()
            .map(|row| Selected {
                row,
                columns: &columns,
            })
            .collect();
        // A single row's `data` is an object, and a list's is an array even
        // when it holds one row.
        match selected.as_slice() {
            [single] if T::SHAPE == Shape::Row => {
                write_document(
                    out,
                    document,
                    output.dry_run,
                    single,
                    total,
                    summary,
                    failures,
                )?;
            }
            list => write_document(
                out,
                document,
                output.dry_run,
                &list,
                total,
                summary,
                failures,
            )?,
        }
    } else if failures.is_some() && rows.is_empty() {
        // A batch in which no id worked prints no table, so the caller sees
        // only its failures on standard error.
    } else {
        if let Some(summary) = summary {
            writeln!(out, "{summary}")?;
        }
        table::write(out, &columns, &rows)?;
        listing::write_footer(out, value)?;
        if let Some(note) = value.note() {
            writeln!(out, "{note}")?;
        }
        listing::write_next(out, output.command.as_mut(), value)?;
    }
    out.flush()
}
