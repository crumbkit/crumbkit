//! `write_result`, which writes a result that is not a failure in the
//! format that the output flags choose, within the output budget
//! (ADR-0046).

use std::io::{self, Write};

use serde_json::Value;

use super::budget::{self, Cut, Over, OverBudget};
use super::dry_run::DRY_RUN_LINE;
use super::envelope::{Extras, Failures, Selected, SelectedRows, write_document};
use super::failure::PreparedFailure;
use super::table::Table;
use super::{Output, listing, prose};
use crate::{Code, Document, Row, Rows, Shape, Spelling, codes};

/// What [`write_result`] printed of a result.
#[derive(Debug)]
pub(super) enum Answer {
    /// The whole result.
    Whole,
    /// The first rows of a listing or a list, marked as cut short with this
    /// code.
    Cut(&'static Code),
    /// The first rows of a batch. The failure of the rows left out follows
    /// the batch's own failures.
    Partial(OverBudget),
    /// Nothing, because the result does not fit in the output budget and its
    /// document does not let its first answer print. The failure prints in
    /// its place.
    Refused(OverBudget),
}

/// Writes `value` to `out` in the format that `output` chooses, within the
/// bytes that [`Output::budget`] has left, then flushes `out`. It returns
/// what it printed and the number of bytes it wrote.
///
/// `failures` holds the failures of a batch or a prose result, which the
/// JSON document holds under `failures`, and is `None` for every other
/// result. A result that fits prints whole. Otherwise a listing or a list
/// prints the most rows that fit, marked as cut short, and a batch prints
/// the most rows that fit and always its first. A single row or a prose
/// result prints nothing, unless its document always prints its first
/// answer, in which case it prints whole. The line that marks a cut in the
/// JSON of a document without the envelope goes to `err`, as does a debug
/// build's warning about a failure.
///
/// The text of a bytes result is the bytes unchanged. They print when they
/// fit, or when the document always prints its first answer, and are
/// refused otherwise, because a file cut short would look whole.
///
/// A serialization error becomes an `io::Error` of kind `InvalidData`, so it
/// is reported as a failed write. The text of a dry run ends with
/// [`DRY_RUN_LINE`], except the text of a bytes result, whose line
/// `finish` writes to standard error.
pub(super) fn write_result<T: Rows>(
    out: &mut impl Write,
    err: &mut impl Write,
    output: &mut Output,
    spelling: Spelling,
    document: Document<T>,
    value: &T,
    failures: Option<&[PreparedFailure<'_>]>,
) -> io::Result<(Answer, usize)> {
    let budget = output.budget();
    let code = document
        .over_budget_code()
        .unwrap_or(&codes::OUTPUT_OVER_BUDGET);
    if let Some(bytes) = value.bytes().filter(|_| !output.json) {
        // The text of bytes is their own length, so it is measured without
        // a buffer.
        if bytes.len() > budget && !document.always_prints_first() {
            let over = OverBudget {
                code,
                budget,
                over: Over::Whole,
            };
            return Ok((Answer::Refused(over), 0));
        }
        out.write_all(bytes)?;
        out.flush()?;
        return Ok((Answer::Whole, bytes.len()));
    }
    let next = if output.json {
        None
    } else {
        listing::next_line(output.command.as_mut(), value)
    };
    let output = &*output;
    let columns = columns(output, document, value);
    let values = values(&columns, value)?;
    let mut prepared = Prepared::new(output, document, value, &columns, &values, next)?;
    let listed = failures.map(|listed| Failures { listed, over: None });
    let rows = value.rows().len();
    let whole = budget::buffered(budget, |buffer| prepared.write(buffer, rows, None, listed))?;
    if let Some(bytes) = whole {
        out.write_all(&bytes)?;
        out.flush()?;
        return Ok((Answer::Whole, bytes.len()));
    }
    match T::SHAPE {
        Shape::Listing | Shape::List if rows > 0 => {
            prepared.cut(out, err, spelling.spell(code), code, budget)
        }
        Shape::Batch if rows > 1 => {
            let listed = failures.unwrap_or_default();
            prepared.partial(out, err, spelling, listed, code, budget)
        }
        Shape::Listing | Shape::List | Shape::Batch => {
            let bytes = prepared.measured_write(out, rows, None, listed)?;
            Ok((Answer::Whole, bytes))
        }
        _ if document.always_prints_first() => {
            let bytes = prepared.measured_write(out, rows, None, listed)?;
            Ok((Answer::Whole, bytes))
        }
        shape => {
            let over = if matches!(shape, Shape::Prose | Shape::Bytes) {
                Over::Whole
            } else {
                Over::Row
            };
            Ok((Answer::Refused(OverBudget { code, budget, over }), 0))
        }
    }
}

/// Returns the columns that the table and the compact JSON show, or an
/// empty list, which does not allocate, when the result prints whole with
/// every field or is a prose result.
fn columns<'a, T: Rows>(output: &'a Output, document: Document<T>, value: &T) -> Vec<&'a str> {
    let unwrapped = output.json && !document.has_envelope();
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
fn values<T: Rows>(columns: &[&str], value: &T) -> io::Result<Vec<Value>> {
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

/// A result ready to write with any number of its first rows: each part
/// that would allocate is built once, so measuring the result with each row
/// count allocates nothing more.
#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag is a separate choice of the caller's command line, which the writers read"
)]
struct Prepared<'a, T> {
    /// The document that the result prints as with `--json`.
    document: Document<T>,
    /// The result.
    value: &'a T,
    /// Whether the caller passed `--json`.
    json: bool,
    /// Whether the run is a dry run.
    dry_run: bool,
    /// Whether the JSON document holds every field of each row.
    full: bool,
    /// Whether the caller passed `--fields`.
    fields: bool,
    /// The columns that the compact JSON shows.
    columns: &'a [&'a str],
    /// Each row serialized once, or nothing when there are no columns.
    values: &'a [Value],
    /// The text table, when the result prints one.
    table: Option<Table<'a>>,
    /// The escaped text of a prose result in text.
    prose: Option<String>,
    /// The rendered call and the phrase of the `Next:` line.
    next: Option<(String, &'a str)>,
}

impl<'a, T: Rows> Prepared<'a, T> {
    /// Builds the parts of `value` that its format writes.
    fn new(
        output: &Output,
        document: Document<T>,
        value: &'a T,
        columns: &'a [&'a str],
        values: &'a [Value],
        next: Option<(String, &'a str)>,
    ) -> io::Result<Self> {
        let text = !output.json;
        let prose = match value.prose() {
            Some(prose) if text => Some(prose::text(prose)?),
            _ => None,
        };
        let table = (text && value.prose().is_none()).then(|| Table::new(columns, values));
        Ok(Self {
            document,
            value,
            json: output.json,
            dry_run: output.dry_run,
            full: output.full,
            fields: !output.fields.is_empty(),
            columns,
            values,
            table,
            prose,
            next,
        })
    }

    /// Writes the result with its first `shown` rows to `out`, marked with
    /// `cut` when the budget cut it short, then the line of a dry run in
    /// text, and flushes `out`.
    fn write(
        &mut self,
        out: &mut impl Write,
        shown: usize,
        cut: Option<&Cut>,
        failures: Option<Failures<'_>>,
    ) -> io::Result<()> {
        self.write_value(out, shown, cut, failures)?;
        if self.dry_run && !self.json {
            writeln!(out, "{DRY_RUN_LINE}")?;
        }
        out.flush()
    }

    /// Measures the result as [`Prepared::write`] writes it, then writes
    /// it, and returns the number of bytes.
    fn measured_write(
        &mut self,
        out: &mut impl Write,
        shown: usize,
        cut: Option<&Cut>,
        failures: Option<Failures<'_>>,
    ) -> io::Result<usize> {
        let bytes = budget::measure(|counter| self.write(counter, shown, cut, failures))?;
        self.write(out, shown, cut, failures)?;
        Ok(bytes)
    }

    /// Writes the most first rows of a listing or a list that fit in
    /// `budget`, marked as cut short with `code`, which `spelled` spells.
    /// A document that always prints its first answer prints at least one
    /// row.
    fn cut(
        &mut self,
        out: &mut impl Write,
        err: &mut impl Write,
        spelled: String,
        code: &'static Code,
        budget: usize,
    ) -> io::Result<(Answer, usize)> {
        let rows = self.value.rows().len();
        let cut = Cut {
            code: spelled,
            budget,
        };
        let low = usize::from(self.document.always_prints_first());
        let mut fits = |shown| {
            let bytes = budget::measure(|counter| self.write(counter, shown, Some(&cut), None))?;
            Ok(bytes <= budget)
        };
        let shown = budget::largest(low, rows - 1, &mut fits)?;
        if shown == 0 && !fits(0)? {
            // Even the result without rows passes the budget, as a long
            // summary can, so the result is refused as a single row is.
            let over = OverBudget {
                code,
                budget,
                over: Over::Row,
            };
            return Ok((Answer::Refused(over), 0));
        }
        let bytes = self.measured_write(out, shown, Some(&cut), None)?;
        if self.json && !self.document.has_envelope() {
            // A document without the envelope has no `cut` field, so the line
            // goes to standard error, and a failed write there goes
            // unreported.
            let _ = self.write_cut_line(err, shown, &cut);
        }
        Ok((Answer::Cut(code), bytes))
    }

    /// Writes the most first rows of a batch that fit in `budget`, and
    /// always its first, with the failure of the rows left out after the
    /// batch's own failures in the JSON document.
    fn partial(
        &mut self,
        out: &mut impl Write,
        err: &mut impl Write,
        spelling: Spelling,
        listed: &[PreparedFailure<'_>],
        code: &'static Code,
        budget: usize,
    ) -> io::Result<(Answer, usize)> {
        let rows = self.value.rows().len();
        let in_document = self.json && self.document.has_envelope();
        // Each count is measured with the failure of every row left out,
        // whose numbers are the largest, so the failure that prints is never
        // longer than the one measured.
        let most = OverBudget {
            code,
            budget,
            over: Over::Batch(rows),
        };
        let most = in_document.then(|| PreparedFailure::new(&most, spelling, None, err));
        let shown = budget::largest(1, rows - 1, |shown| {
            let failures = Failures {
                listed,
                over: most.as_ref(),
            };
            let bytes =
                budget::measure(|counter| self.write(counter, shown, None, Some(failures)))?;
            Ok(bytes <= budget)
        })?;
        let over = OverBudget {
            code,
            budget,
            over: Over::Batch(rows - shown),
        };
        let bytes = {
            let prepared = in_document.then(|| PreparedFailure::new(&over, spelling, None, err));
            let failures = Failures {
                listed,
                over: prepared.as_ref(),
            };
            self.measured_write(out, shown, None, Some(failures))?
        };
        Ok((Answer::Partial(over), bytes))
    }

    /// Writes the line that ends the text of a result that the budget cut
    /// short to `out`.
    fn write_cut_line(&self, out: &mut impl Write, shown: usize, cut: &Cut) -> io::Result<()> {
        let total = self.value.total().unwrap_or(self.value.rows().len());
        let listing = T::SHAPE == Shape::Listing;
        let description = self.value.description();
        budget::write_cut_line(out, shown, total, description, cut.budget, listing)
    }

    /// Writes the result as [`Prepared::write`] does, without the line of a
    /// dry run.
    fn write_value(
        &mut self,
        out: &mut impl Write,
        shown: usize,
        cut: Option<&Cut>,
        failures: Option<Failures<'_>>,
    ) -> io::Result<()> {
        let value = self.value;
        let rows = value.rows();
        let all = shown >= rows.len();
        if self.json && !self.document.has_envelope() {
            return self.write_unwrapped(out, shown, all);
        }
        let extras = Extras {
            total: value.total(),
            summary: value.summary(),
            cut,
            failures,
        };
        if let Some(bytes) = value.bytes() {
            return if self.json {
                write_document(out, self.document, self.dry_run, value, &extras)
            } else {
                out.write_all(bytes)
            };
        }
        if value.prose().is_some() {
            return match &self.prose {
                Some(text) => out.write_all(text.as_bytes()),
                None => write_document(out, self.document, self.dry_run, value, &extras),
            };
        }
        if self.json && self.full {
            // Serializing the rows directly keeps every field in its
            // declaration order.
            return match rows.get(..shown) {
                Some(first) if !all => {
                    write_document(out, self.document, self.dry_run, &first, &extras)
                }
                _ => write_document(out, self.document, self.dry_run, value, &extras),
            };
        }
        if self.json {
            // A single row's `data` is an object, and a list's is an array
            // even when it holds one row.
            return match self.values {
                [single] if T::SHAPE == Shape::Row => {
                    let selected = Selected {
                        row: single,
                        columns: self.columns,
                    };
                    write_document(out, self.document, self.dry_run, &selected, &extras)
                }
                list => {
                    let selected = SelectedRows {
                        rows: list.get(..shown).unwrap_or(list),
                        columns: self.columns,
                    };
                    write_document(out, self.document, self.dry_run, &selected, &extras)
                }
            };
        }
        if failures.is_some() && shown.min(rows.len()) == 0 {
            // A batch in which no id worked prints no table, so the caller
            // sees only its failures on standard error.
            return Ok(());
        }
        if let Some(summary) = value.summary() {
            writeln!(out, "{summary}")?;
        }
        if let Some(table) = &mut self.table {
            table.write(out, shown)?;
        }
        if let Some(cut) = cut {
            self.write_cut_line(out, shown, cut)?;
        } else {
            listing::write_footer(out, value)?;
            listing::write_empty_note(out, value, self.document.empty_note())?;
        }
        if let Some(note) = value.note() {
            writeln!(out, "{note}")?;
        }
        if let Some((call, phrase)) = &self.next {
            writeln!(out, "Next: {call} {phrase}")?;
        }
        Ok(())
    }

    /// Writes the JSON of a document without the envelope with its first
    /// `shown` rows, or the whole result when `all` is set (ADR-0016).
    ///
    /// Without `--fields`, the whole result prints as its type serializes it,
    /// so a command prints an existing document unchanged, and its first
    /// rows print as an array. With `--fields`, only those fields of each row
    /// print, in that order, as an object for a single row and as an array
    /// otherwise.
    fn write_unwrapped(&self, out: &mut impl Write, shown: usize, all: bool) -> io::Result<()> {
        let rows = self.value.rows();
        if self.fields {
            match self.values {
                [single] if T::SHAPE == Shape::Row => {
                    let selected = Selected {
                        row: single,
                        columns: self.columns,
                    };
                    serde_json::to_writer(&mut *out, &selected)?;
                }
                list => {
                    let selected = SelectedRows {
                        rows: list.get(..shown).unwrap_or(list),
                        columns: self.columns,
                    };
                    serde_json::to_writer(&mut *out, &selected)?;
                }
            }
        } else {
            match rows.get(..shown) {
                Some(first) if !all => serde_json::to_writer(&mut *out, first)?,
                _ => serde_json::to_writer(&mut *out, self.value)?,
            }
        }
        out.write_all(b"\n")
    }
}
