//! `write_result`, which writes a result that is not a failure in the
//! format that the output flags choose, within the output budget
//! (ADR-0046).

use std::ffi::OsString;
use std::io::{self, Write};

use clap::Command;
use serde::Serialize;
use serde_json::Value;

use super::budget::{self, Cut, Over, OverBudget};
use super::columns::{columns, values};
use super::envelope::{Extras, Failures, Selected, SelectedRows, write_document};
use super::failure::PreparedFailure;
use super::layout::JsonLayout;
use super::table::Table;
use super::{Output, children, layout, lines, listing, problems, prose};
use crate::age::TextCells;
use crate::help::rename::Renames;
use crate::sink::role::{Deliver, Message, Role};
use crate::view::View;
use crate::{Code, Document, Row, Rows, Shape, Spelling, codes};

mod batch;
mod cut;
mod fitted;
pub(super) mod headed;
mod rerun;

/// What [`write_result`] printed of a result.
#[derive(Debug)]
pub(super) enum Answer {
    /// The whole result.
    Whole,
    /// The first rows of a listing or a list, marked as cut short with this
    /// code.
    Cut(&'static Code),
    /// The first rows of a batch, in a table, a JSON document or the text of
    /// a headed batch. The failure of the keys left out follows the batch's
    /// own failures.
    Partial(OverBudget),
    /// Nothing, because the result does not fit in the output budget and its
    /// document does not let its first answer print. The failure prints in
    /// its place.
    Refused(OverBudget),
    /// No rows of a batch, because its first answer passes the output budget
    /// alone and its document does not let its first answer print.
    /// `first` is that answer's failure, which prints among the batch's own
    /// failures under the key at `position`, and `left_out` is the failure
    /// that names the keys after it, when there are any.
    RefusedFirst {
        position: usize,
        first: OverBudget,
        left_out: Option<OverBudget>,
    },
}

/// Delivers `value` to `sink` as the result in the format that `output`
/// chooses, within the bytes that [`Output::budget`] has left. It returns
/// what it printed and the number of bytes it wrote.
///
/// `failures` holds the failures of a batch or a prose result, which the
/// JSON document holds under `failures`, and is `None` for every other
/// result. `problems` is the number of rows that report a problem, which
/// the text's note counts when there is no failure (ADR-0050).
///
/// A result that fits prints whole. Otherwise a listing or a list prints the
/// most rows that fit, marked as cut short. A batch prints the most rows
/// that fit, followed by one failure that names the keys left out. Its
/// first row prints even past the budget when its document always prints
/// its first answer, and is refused with its own failure when it passes
/// the budget alone otherwise. A batch that has keys it never
/// fetched prints those failures even when its rows fit. A single row or a prose
/// result prints nothing, unless its document always prints its first
/// answer, in which case it prints whole. After the result, the lines that
/// describe it are delivered as a note: in text, a listing's footer or
/// empty note, the cut line, the note of the rows that report a problem,
/// the tool's note and the `Next:` line, and with `--json`, the cut line
/// of a document without the envelope (ADR-0050). A debug build's warning
/// about a failure is delivered as a debug warning. A text table fits the
/// width that `sink` gives the result (ADR-0121, ADR-0128).
///
/// The text of a bytes result is the bytes unchanged. They print when they
/// fit, or when the document always prints its first answer, and are
/// refused otherwise, because a file cut short would look whole. A headed
/// batch's text prints as [`headed::write`] describes.
///
/// A serialization error becomes an `io::Error` of kind `InvalidData`, so it
/// is reported as a failed write. `finish` delivers the line of a dry run
/// itself, after a batch's failures.
pub(super) fn write_result<T: Rows>(
    sink: &mut dyn Deliver,
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
    if let Some(bytes) = value.bytes().filter(|_| !output.is_json()) {
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
        sink.deliver(Role::Result, bytes)?;
        return Ok((Answer::Whole, bytes.len()));
    }
    // The problem note names the exit, so it prints only without a failure.
    let problems = if failures.is_some_and(|listed| !listed.is_empty()) {
        0
    } else {
        problems
    };
    if let Some(bytes_of) = document.headed_bytes().filter(|_| !output.is_json()) {
        let first_always = document.always_prints_first();
        return headed::write(sink, output, value, bytes_of, problems, first_always);
    }
    // `Prepared` borrows the settings for the whole write, and the `Next:`
    // line and the failure of a batch's left-out keys render their calls
    // against the command, so the write borrows the two apart.
    let (settings, command) = Settings::split(output, sink.width(Role::Result));
    write_rows(
        sink, &settings, command, spelling, document, value, failures, problems,
    )
}

/// The settings of a run that a result prints with, borrowed apart from the
/// command tree, so that a write reads them while it renders a call against
/// the tree.
struct Settings<'a> {
    /// The view that the view flags selected.
    view: &'a View,
    /// The words that the caller typed after the program name.
    words: &'a [OsString],
    /// Whether the run is a dry run.
    dry_run: bool,
    /// The layout of a whole JSON document.
    layout: JsonLayout,
    /// The most bytes that the run prints on standard output.
    budget: usize,
    /// The tool's own codes for crumbkit's failures.
    renames: Renames,
    /// The number of columns that the sink gives the result, which a text
    /// table fits, or `None` when the table prints unfitted.
    width: Option<usize>,
}

impl<'a> Settings<'a> {
    /// Borrows the settings of `output` and, apart from them, its command
    /// tree, with the `width` that the sink gives the result.
    fn split(output: &'a mut Output, width: Option<usize>) -> (Self, &'a mut Command) {
        let settings = Self {
            view: &output.view,
            words: &output.words,
            dry_run: output.dry_run,
            layout: output.json_layout,
            budget: output.budget,
            renames: output.renames,
            width,
        };
        (settings, &mut output.command)
    }

    /// Reports whether the caller asked for JSON with `--json`.
    const fn is_json(&self) -> bool {
        self.view.is_json()
    }
}

/// Writes a result that is not bytes in text, as [`write_result`] does.
/// `command` is the tool's command, which the `Next:` line and the failure
/// of a batch's left-out keys render their calls against.
#[expect(
    clippy::too_many_arguments,
    reason = "write_result passes its own arguments on, with the command borrowed apart from the settings"
)]
fn write_rows<T: Rows>(
    sink: &mut dyn Deliver,
    settings: &Settings<'_>,
    command: &mut Command,
    spelling: Spelling,
    document: Document<T>,
    value: &T,
    failures: Option<&[PreparedFailure<'_>]>,
    problems: usize,
) -> io::Result<(Answer, usize)> {
    let budget = settings.budget;
    let code = settings.renames.printed(&codes::OUTPUT_OVER_BUDGET);
    let next = if settings.is_json() {
        None
    } else {
        listing::next_line(&mut *command, value)
    };
    let columns = columns(settings.view, document, value.prose().is_some());
    // The serialized rows become the cells of the text table without
    // `--json` and the selected fields of the JSON document with it, and a
    // run prints only one of the two. The rows are serialized once, so an
    // `Age` writes its compact age when the run prints text.
    let values = {
        let _cells = (!settings.is_json()).then(TextCells::enter);
        values(&columns, value)?
    };
    let mut prepared = Prepared::new(settings, document, value, &columns, &values, next, problems)?;
    let listed = failures.map(Failures::of);
    let rows = value.rows().len();
    // A batch that never fetched some of its keys ends with their failure
    // even when its rows fit.
    let whole = if value.left_out(rows).is_empty() {
        budget::buffered(budget, |buffer| prepared.write(buffer, rows, None, listed))?
    } else {
        None
    };
    if let Some(mut bytes) = whole {
        prepared.write_measured(sink, &bytes, rows, listed)?;
        let written = bytes.len();
        // The buffer of the result is free again, so the notes reuse it.
        prepared.send_notes(sink, &mut bytes, rows, None);
        return Ok((Answer::Whole, written));
    }
    match T::SHAPE {
        Shape::Listing | Shape::List if rows > 0 => {
            prepared.cut(sink, spelling.spell(code), code, budget)
        }
        Shape::Batch => {
            let listed = failures.unwrap_or_default();
            let rerun = rerun::Rerun::find(command, settings.words, value);
            let left_out = batch::LeftOut {
                spelling,
                command,
                code,
                budget,
                rerun,
            };
            prepared.partial(sink, listed, left_out)
        }
        Shape::Listing | Shape::List => {
            let bytes = prepared.measured_write(sink, rows, None, listed)?;
            prepared.send_notes(sink, &mut Vec::new(), rows, None);
            Ok((Answer::Whole, bytes))
        }
        _ if document.always_prints_first() => {
            let bytes = prepared.measured_write(sink, rows, None, listed)?;
            prepared.send_notes(sink, &mut Vec::new(), rows, None);
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

/// A result ready to write with any number of its first rows: each part
/// that would allocate is built once, so measuring the result with each row
/// count allocates nothing more.
struct Prepared<'a, T> {
    /// The document that the result prints as with `--json`.
    document: Document<T>,
    /// The result.
    value: &'a T,
    /// The view that the view flags selected.
    view: &'a View,
    /// Whether the run is a dry run.
    dry_run: bool,
    /// The layout of the JSON document.
    layout: JsonLayout,
    /// Whether the JSON document holds every field of each row.
    full: bool,
    /// The columns that the compact JSON shows.
    columns: &'a [&'a str],
    /// Each row serialized once, or nothing when there are no columns.
    values: &'a [Value],
    /// The text table, when the result prints one.
    table: Option<Table<'a>>,
    /// The number of columns that the sink gives the result, or `None`
    /// when the table prints unfitted.
    width: Option<usize>,
    /// Whether the table is fitted to `width`, which is only while
    /// [`Prepared::write_fitted`] writes the result.
    fitting: bool,
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
        settings: &Settings<'a>,
        document: Document<T>,
        value: &'a T,
        columns: &'a [&'a str],
        values: &'a [Value],
        next: Option<(String, &'a str)>,
        problems: usize,
    ) -> io::Result<Self> {
        let text = !settings.is_json();
        let prose = match value.prose() {
            Some(prose) if text => Some(prose::text(prose)?),
            _ => None,
        };
        let lines = if lines::chosen::<T::Row>(settings.view) {
            let rows = value.rows().iter().map(lines::line);
            Some(rows.collect::<io::Result<Vec<String>>>()?)
        } else {
            None
        };
        let table = (text && value.prose().is_none() && lines.is_none()).then(|| {
            let children = children::blocks(settings.view, value.rows());
            Table::new(columns, values, settings.view.names().is_none())
                .keeping_whole(columns, T::Row::UNCUT_COLUMNS)
                .with_children(children)
        });
        Ok(Self {
            document,
            value,
            view: settings.view,
            dry_run: settings.dry_run,
            layout: settings.layout,
            // A headed document and a row type that prints whole hold
            // every field in JSON unless `--fields` names some.
            full: settings.view.is_full()
                || ((document.is_headed() || T::Row::WHOLE) && settings.view.names().is_none()),
            columns,
            values,
            table,
            width: settings.width,
            fitting: false,
            lines,
            prose,
            next,
            problems,
        })
    }

    /// Writes the result with its first `shown` rows to `out`, marked with
    /// `cut` when the budget cut it short, and flushes `out`. The lines that
    /// describe the result are delivered as a note through
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

    /// Delivers the lines that describe the result with its first `shown`
    /// rows to `sink` as a note, after the result (ADR-0050).
    ///
    /// The lines are rendered into `buffer`, which is cleared first, and
    /// are delivered as one piece, so a reader that merges the two streams
    /// never sees a note split by other output. A note is the last thing to
    /// report, so a failure to deliver it goes unreported.
    fn send_notes(
        &self,
        sink: &mut dyn Deliver,
        buffer: &mut Vec<u8>,
        shown: usize,
        cut: Option<&Cut>,
    ) {
        buffer.clear();
        // A `Vec` takes every byte, so writing the notes cannot fail.
        let _ = self.write_notes(buffer, shown, cut);
        if !buffer.is_empty() {
            let _ = sink.deliver(Role::Message(Message::Note), buffer);
        }
    }

    /// Sends the lines that describe a batch whose first `shown` rows print
    /// and whose later answers the output budget leaves out, as
    /// [`Prepared::send_notes`] does. The failure of the answers left out
    /// decides the exit, so the note of the rows that report a problem,
    /// which names the exit, does not print, and the tool's note does.
    fn send_notes_before_left_out(&mut self, sink: &mut dyn Deliver, shown: usize) {
        self.problems = 0;
        self.send_notes(sink, &mut Vec::new(), shown, None);
    }

    /// Writes the lines that describe the result to `out`: in text, the cut
    /// line, or else the footer and the empty note, then the note of the
    /// rows that report a problem, the tool's note and the `Next:` line; with `--json`, only the cut line of a document
    /// without the envelope, which has no `cut` field.
    fn write_notes(&self, out: &mut impl Write, shown: usize, cut: Option<&Cut>) -> io::Result<()> {
        let value = self.value;
        if self.view.is_json() {
            if let Some(cut) = cut.filter(|_| !self.document.get_envelope().wraps()) {
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
        if self.view.is_json() && !self.document.get_envelope().wraps() {
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
            return if self.view.is_json() {
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
        if self.view.is_json() && self.full {
            // Serializing the rows directly keeps every field in its
            // declaration order.
            return match rows.get(..shown) {
                Some(first) if !all => self.envelope(out, &first, &extras),
                _ => self.envelope(out, value, &extras),
            };
        }
        if self.view.is_json() {
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
            // sees only its failure messages.
            return Ok(());
        }
        if let Some(summary) = value.summary() {
            // A summary whose `Display` writes no text prints no line, so
            // the header stays the first line (ADR-0139).
            let line = summary.to_string();
            if !line.is_empty() {
                writeln!(out, "{line}")?;
            }
        }
        if let Some(table) = &mut self.table {
            table.write(out, shown, self.width.filter(|_| self.fitting))?;
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
        if self.view.names().is_none() {
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
