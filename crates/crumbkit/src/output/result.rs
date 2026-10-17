//! `write_result`, which writes a result that is not a failure in the
//! format that the output flags choose, within the output budget
//! (ADR-0046).

use std::io::{self, Write};

use clap::Command;
use serde::Serialize;
use serde_json::Value;

use super::budget::{self, Cut, Over, OverBudget};
use super::envelope::{Extras, Failures, Selected, SelectedRows, write_document};
use super::failure::PreparedFailure;
use super::table::Table;
use super::{Output, layout, lines, listing, problems, prose};
use crate::form::FailureForm;
use crate::{Code, Document, JsonLayout, Row, Rows, Shape, Spelling, codes};

mod batch;
mod cut;
mod headed;
mod in_order;

/// What [`write_result`] printed of a result.
#[derive(Debug)]
pub(super) enum Answer {
    /// The whole result.
    Whole,
    /// The first rows of a listing or a list, marked as cut short with this
    /// code.
    Cut(&'static Code),
    /// The first rows of a batch. The failure of the keys left out follows
    /// the batch's own failures.
    Partial(OverBudget),
    /// Nothing, because the result does not fit in the output budget and its
    /// document does not let its first answer print. The failure prints in
    /// its place.
    Refused(OverBudget),
    /// The whole rows of a headed batch's text that fit, with the failure of
    /// each row too large alone, by position, and of the rows left out.
    Headed {
        oversize: Vec<(usize, OverBudget)>,
        left_out: Option<OverBudget>,
    },
}

/// Writes `value` to `out` in the format that `output` chooses, within the
/// bytes that [`Output::budget`] has left, then flushes `out`. It returns
/// what it printed and the number of bytes it wrote.
///
/// `failures` holds the failures of a batch or a prose result, which the
/// JSON document holds under `failures`, and is `None` for every other
/// result. `problems` is the number of rows that report a problem, which
/// the text's note counts when there is no failure (ADR-0050).
///
/// A result that fits prints whole. Otherwise a listing or a list prints the
/// most rows that fit, marked as cut short. A batch prints the most rows
/// that fit and always its first, followed by one failure that names the
/// keys left out. A batch that has keys it never fetched prints that
/// failure even when its rows fit. A single row or a prose
/// result prints nothing, unless its document always prints its first
/// answer, in which case it prints whole. After `out` is flushed, the lines
/// that describe the result go to `err`: in text, a listing's footer or
/// empty note, the cut line, the note of the rows that report a problem,
/// the tool's note and the `Next:` line, and with
/// `--json`, the cut line of a document without the envelope (ADR-0050). A
/// debug build's warning about a failure also goes to `err`.
///
/// The text of a bytes result is the bytes unchanged. They print when they
/// fit, or when the document always prints its first answer, and are
/// refused otherwise, because a file cut short would look whole. A headed
/// batch's text prints as [`headed::write`] describes.
///
/// A serialization error becomes an `io::Error` of kind `InvalidData`, so it
/// is reported as a failed write. `finish` writes the line of a dry run to
/// standard error itself, after a batch's failures.
#[expect(
    clippy::too_many_arguments,
    reason = "respond reads the failures and counts the problem rows once, and passes both with the result it prints"
)]
pub(super) fn write_result<T: Rows>(
    out: &mut impl Write,
    err: &mut impl Write,
    output: &mut Output,
    spelling: Spelling,
    document: Document<T>,
    value: &T,
    failures: Option<&[PreparedFailure<'_>]>,
    problems: usize,
) -> io::Result<(Answer, usize)> {
    let budget = output.budget();
    // The code is the tool's rename when the guide has one, so the cut,
    // the failure, the usage log and the trace all carry it.
    let code = output.renames.printed(&codes::OUTPUT_OVER_BUDGET);
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
    // The problem note names the exit, so it prints only without a failure.
    let problems = if failures.is_some_and(|listed| !listed.is_empty()) {
        0
    } else {
        problems
    };
    if let Some(bytes_of) = document.headed_bytes().filter(|_| !output.json) {
        return headed::write(out, err, output, document, value, bytes_of, problems);
    }
    // `Prepared` borrows the output flags for the whole write, and the
    // failure of a batch's left-out keys renders its call against the
    // command, so the write takes the command out of `output` and puts it
    // back after.
    let mut command = output.command.take();
    let written = write_rows(
        out,
        err,
        output,
        command.as_mut(),
        spelling,
        document,
        value,
        failures,
        problems,
    );
    output.command = command;
    written
}

/// Writes a result that is not bytes in text, as [`write_result`] does.
/// `command` is the tool's command, which the `Next:` line and the failure
/// of a batch's left-out keys render their calls against.
#[expect(
    clippy::too_many_arguments,
    reason = "write_result passes its own arguments on, with the command taken out of the output flags"
)]
fn write_rows<T: Rows>(
    out: &mut impl Write,
    err: &mut impl Write,
    output: &Output,
    mut command: Option<&mut Command>,
    spelling: Spelling,
    document: Document<T>,
    value: &T,
    failures: Option<&[PreparedFailure<'_>]>,
    problems: usize,
) -> io::Result<(Answer, usize)> {
    let budget = output.budget();
    let code = output.renames.printed(&codes::OUTPUT_OVER_BUDGET);
    let next = if output.json {
        None
    } else {
        listing::next_line(command.as_deref_mut(), value)
    };
    let columns = columns(output, document, value);
    let values = values(&columns, value)?;
    let mut prepared = Prepared::new(output, document, value, &columns, &values, next, problems)?;
    let listed = failures.map(|listed| Failures {
        listed,
        over: None,
        form: output.failure_form,
    });
    let rows = value.rows().len();
    // A batch that never fetched some of its keys ends with their failure
    // even when its rows fit.
    let whole = if value.left_out(rows).is_empty() {
        budget::buffered(budget, |buffer| prepared.write(buffer, rows, None, listed))?
    } else {
        None
    };
    if let Some(mut bytes) = whole {
        out.write_all(&bytes)?;
        out.flush()?;
        let written = bytes.len();
        // The buffer of the result is free again, so the notes reuse it.
        prepared.send_notes(err, &mut bytes, rows, None);
        return Ok((Answer::Whole, written));
    }
    match T::SHAPE {
        Shape::Listing | Shape::List if rows > 0 => {
            prepared.cut(out, err, spelling.spell(code), code, budget)
        }
        Shape::Batch => {
            let listed = failures.unwrap_or_default();
            let left_out = batch::LeftOut {
                spelling,
                command,
                code,
                budget,
            };
            prepared.partial(out, err, listed, left_out)
        }
        Shape::Listing | Shape::List => {
            let bytes = prepared.measured_write(out, rows, None, listed)?;
            prepared.send_notes(err, &mut Vec::new(), rows, None);
            Ok((Answer::Whole, bytes))
        }
        _ if document.always_prints_first() => {
            let bytes = prepared.measured_write(out, rows, None, listed)?;
            prepared.send_notes(err, &mut Vec::new(), rows, None);
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
/// every field, is a prose result, or prints each row's own text line.
/// Unwrapped or headed JSON holds every field unless `--fields` names some
/// (ADR-0062).
fn columns<'a, T: Rows>(output: &'a Output, document: Document<T>, value: &T) -> Vec<&'a str> {
    if lines::chosen::<T::Row>(output) {
        return Vec::new();
    }
    let unwrapped = output.json && (!document.has_envelope() || document.is_headed());
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
    /// The layout of the JSON document, or `None` for one line.
    layout: Option<JsonLayout>,
    /// The guide's failure form, which writes each entry of `failures`.
    form: Option<&'static dyn FailureForm>,
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
    /// The text line of each row with its line feed, when the rows print
    /// their own lines in place of the table (ADR-0064).
    lines: Option<Vec<String>>,
    /// The escaped text of a prose result in text.
    prose: Option<String>,
    /// The rendered call and the phrase of the `Next:` line.
    next: Option<(String, &'a str)>,
    /// The number of rows that report a problem, which the text's note
    /// counts, or 0 when the run has a failure.
    problems: usize,
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
        problems: usize,
    ) -> io::Result<Self> {
        let text = !output.json;
        let prose = match value.prose() {
            Some(prose) if text => Some(prose::text(prose)?),
            _ => None,
        };
        let lines = if lines::chosen::<T::Row>(output) {
            let rows = value.rows().iter().map(lines::line);
            Some(rows.collect::<io::Result<Vec<String>>>()?)
        } else {
            None
        };
        let table = (text && value.prose().is_none() && lines.is_none())
            .then(|| Table::new(columns, values));
        Ok(Self {
            document,
            value,
            json: output.json,
            dry_run: output.dry_run,
            layout: output.json_layout,
            form: output.failure_form,
            full: output.full || (document.is_headed() && output.fields.is_empty()),
            fields: !output.fields.is_empty(),
            columns,
            values,
            table,
            lines,
            prose,
            next,
            problems,
        })
    }

    /// Writes the result with its first `shown` rows to `out`, marked with
    /// `cut` when the budget cut it short, and flushes `out`. The lines that
    /// describe the result go to standard error through
    /// [`Prepared::send_notes`].
    fn write(
        &mut self,
        out: &mut impl Write,
        shown: usize,
        cut: Option<&Cut>,
        failures: Option<Failures<'_>>,
    ) -> io::Result<()> {
        self.write_value(out, shown, cut, failures)?;
        out.flush()
    }

    /// Writes the lines that describe the result with its first `shown`
    /// rows to `err`, after the result is written and flushed (ADR-0050).
    ///
    /// The lines are rendered into `buffer`, which is cleared first, and
    /// reach `err` in one write, so a reader that merges the two streams
    /// never sees a note split by other output. A failed write to `err`
    /// goes unreported, because standard error is the last place to report
    /// anything.
    fn send_notes(
        &self,
        err: &mut impl Write,
        buffer: &mut Vec<u8>,
        shown: usize,
        cut: Option<&Cut>,
    ) {
        buffer.clear();
        // A `Vec` takes every byte, so writing the notes cannot fail.
        let _ = self.write_notes(buffer, shown, cut);
        if !buffer.is_empty() {
            let _ = err.write_all(buffer).and_then(|()| err.flush());
        }
    }

    /// Writes the lines that describe the result to `out`: in text, the cut
    /// line, or else the footer and the empty note, then the note of the
    /// rows that report a problem, the tool's note and the `Next:` line; with `--json`, only the cut line of a document
    /// without the envelope, which has no `cut` field.
    fn write_notes(&self, out: &mut impl Write, shown: usize, cut: Option<&Cut>) -> io::Result<()> {
        let value = self.value;
        if self.json {
            if let Some(cut) = cut.filter(|_| !self.document.has_envelope()) {
                self.write_cut_line(out, shown, cut)?;
            }
            return Ok(());
        }
        if let Some(cut) = cut {
            self.write_cut_line(out, shown, cut)?;
        } else {
            listing::write_footer(out, value)?;
            listing::write_empty_note(out, value, self.document.empty_note())?;
        }
        if self.problems > 0 {
            writeln!(
                out,
                "{}",
                problems::note(self.problems, value.description())
            )?;
        }
        if let Some(note) = value.note() {
            writeln!(out, "{note}")?;
        }
        if let Some((call, phrase)) = &self.next {
            writeln!(out, "Next: {call} {phrase}")?;
        }
        Ok(())
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

    /// Writes `data` with `extras` in the document's envelope, with the
    /// run's dry-run mark and the guide's JSON layout.
    fn envelope<D: Serialize, M: Serialize>(
        &self,
        out: &mut impl Write,
        data: &D,
        extras: &Extras<'_, M>,
    ) -> io::Result<()> {
        write_document(out, self.document, self.dry_run, self.layout, data, extras)
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
        if let Some(failures) =
            failures.filter(|_| self.json && self.document.prints_failures_in_order())
        {
            return self.write_in_order(out, shown, failures);
        }
        if self.json && !self.document.has_envelope() {
            return self.write_unwrapped(out, shown, all);
        }
        let extras = Extras {
            total: value.total(),
            truncated: (T::SHAPE == Shape::Listing)
                .then(|| cut.is_some() || listing::leaves_out(value)),
            summary: value.summary(),
            cut,
            failures,
        };
        if let Some(bytes) = value.bytes() {
            return if self.json {
                self.envelope(out, value, &extras)
            } else {
                out.write_all(bytes)
            };
        }
        if value.prose().is_some() {
            return match &self.prose {
                Some(text) => out.write_all(text.as_bytes()),
                None => self.envelope(out, value, &extras),
            };
        }
        if self.json && self.full {
            // Serializing the rows directly keeps every field in its
            // declaration order.
            return match rows.get(..shown) {
                Some(first) if !all => self.envelope(out, &first, &extras),
                _ => self.envelope(out, value, &extras),
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
                    self.envelope(out, &selected, &extras)
                }
                list => {
                    let selected = SelectedRows {
                        rows: list.get(..shown).unwrap_or(list),
                        columns: self.columns,
                    };
                    self.envelope(out, &selected, &extras)
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
        for line in self.lines.iter().flatten().take(shown) {
            out.write_all(line.as_bytes())?;
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
        if !self.fields {
            return match rows.get(..shown) {
                Some(first) if !all => layout::write(out, first, self.layout),
                _ => layout::write(out, self.value, self.layout),
            };
        }
        match self.values {
            [single] if T::SHAPE == Shape::Row => {
                let selected = Selected {
                    row: single,
                    columns: self.columns,
                };
                layout::write(out, &selected, self.layout)
            }
            list => {
                let selected = SelectedRows {
                    rows: list.get(..shown).unwrap_or(list),
                    columns: self.columns,
                };
                layout::write(out, &selected, self.layout)
            }
        }
    }
}
