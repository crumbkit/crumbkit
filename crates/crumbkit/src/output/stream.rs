//! `stream`, which prints the rows of a stream command one line at a time
//! as each arrives (ADR-0041).

use std::borrow::Cow;
use std::io::{self, Stderr, StdoutLock, Write};
use std::ops::ControlFlow;
use std::process::ExitCode;

use clap::Command;

use super::budget::{Over, OverBudget};
use super::dry_run::DRY_RUN_LINE;
use super::envelope::{Extras, Selected, write_document};
use super::failure::{self, PreparedFailure};
use super::{Output, fields, table, unwrapped};
use crate::form::FailureForm;
use crate::help::Exit;
use crate::{AsFailure, Document, Failure, Row, Sink, Spelling, Stream, codes, exit};

#[cfg(test)]
mod tests;

/// Opens the stream of `document` on the process's standard output and
/// standard error, in the format that `output` chooses.
///
/// It behaves as [`stream_to`] does with a sink over the standard streams.
/// Standard output stays locked until the [`Streaming`] is dropped, so
/// another thread of the tool that prints to standard output waits until the
/// stream ends.
///
/// # Errors
///
/// It returns `Err` with exit code 2 after it prints the refusal when
/// `--fields` names a field that `R` does not have. The tool's `main`
/// returns that exit code.
pub fn stream<R: Row>(
    output: &mut Output,
    spelling: Spelling,
    document: Document<Stream<R>>,
) -> Result<Streaming<'_, R, StdoutLock<'static>, Stderr>, ExitCode> {
    let Sink { out, err, .. } = Sink::standard();
    open(out, err, output, spelling, document)
}

/// Opens the stream of `document` on `sink`, in the format that `output`
/// chooses, and returns the [`Streaming`] that prints each item.
///
/// Tests and golden files call it with a sink over in-memory writers, as
/// they call [`finish_to`](crate::finish_to).
///
/// In text, the stream prints the table's header when it opens, then one
/// line for each item. Each column is as wide as its header, two spaces
/// separate the columns, and the last cell is not padded. A cell wider than
/// its header pushes the rest of its line to the right, because the stream
/// cannot know the widths of the items still to come. `--long` adds the long
/// columns, and `--fields` prints exactly the named fields in that order.
///
/// With `--json`, each item prints as one JSON document on its own line,
/// with the `schema` id of `document` and the item under `data`, which makes
/// the output JSON Lines. `--fields` and `--full` shape each item as they
/// shape a single row in [`finish_to`](crate::finish_to). A document from
/// [`Document::without_envelope`] prints each item alone, and its failures
/// print as text on the error writer even with `--json`.
///
/// The stream flushes the output writer after each line, so a reader sees
/// each item as soon as it is printed.
///
/// The stream prints within the bytes that [`Output::budget`] has left when
/// it opens. Its first item always prints. A later item whose line would
/// pass the budget does not print: the stream prints the failure
/// `crumbkit::output::over_budget`, or the document's own over-budget code,
/// in its place, stops, and ends with exit code 1 (ADR-0046). A tool whose
/// stream runs for a long time sets a larger budget with
/// [`Guide::output_budget`](crate::help::Guide::output_budget).
///
/// A tool cannot pass a stream document to [`finish`](crate::finish), and
/// cannot pass a document of one result here:
///
/// ```compile_fail
/// # #[derive(serde::Serialize, schemars::JsonSchema)]
/// # struct Entry { position: u32 }
/// # impl crumbkit::Row for Entry { const COLUMNS: &'static [&'static str] = &["position"]; }
/// use crumbkit::{Document, Error, Output, Spelling, Stream};
///
/// const ENTRIES_STREAM: Document<Stream<Entry>> =
///     Document::new("reading", "entries-stream", 1, "Each entry of a reading list.");
///
/// let result: Result<Vec<Entry>, Error> = Ok(Vec::new());
/// crumbkit::finish(&mut Output::default(), Spelling::COLONS, ENTRIES_STREAM, result);
/// ```
///
/// A tool that streams entries opens the stream, prints each item until the
/// reader stops, and ends the stream with the result of the work:
///
/// ```
/// # #[derive(serde::Serialize, schemars::JsonSchema)]
/// # struct Entry { position: u32 }
/// # impl crumbkit::Row for Entry { const COLUMNS: &'static [&'static str] = &["position"]; }
/// use std::ops::ControlFlow;
///
/// use crumbkit::{Document, Error, Output, Sink, Spelling, Stream};
///
/// const ENTRIES_STREAM: Document<Stream<Entry>> =
///     Document::new("reading", "entries-stream", 1, "Each entry of a reading list.");
///
/// let mut output = Output::default();
/// let mut out = Vec::new();
/// let mut err = Vec::new();
/// let mut sink = Sink::new(&mut out, &mut err);
/// let code = match crumbkit::stream_to(&mut sink, &mut output, Spelling::COLONS, ENTRIES_STREAM) {
///     Ok(mut streaming) => {
///         for position in 1..=3 {
///             if streaming.item(&Entry { position }).is_break() {
///                 break;
///             }
///         }
///         streaming.end(Ok::<(), Error>(()))
///     }
///     Err(code) => code,
/// };
/// assert_eq!(String::from_utf8(out).unwrap(), "POSITION\n1\n2\n3\n");
/// ```
///
/// # Errors
///
/// It returns `Err` with exit code 2 when `--fields` names a field that `R`
/// does not have. The refusal prints as [`finish_to`](crate::finish_to)
/// prints it, and nothing of the stream prints.
pub fn stream_to<'a, R: Row, O: Write, W: Write>(
    sink: &'a mut Sink<O, W>,
    output: &'a mut Output,
    spelling: Spelling,
    document: Document<Stream<R>>,
) -> Result<Streaming<'a, R, &'a mut O, &'a mut W>, ExitCode> {
    open(&mut sink.out, &mut sink.err, output, spelling, document)
}

/// An open stream, which prints each item that the tool passes to
/// [`Streaming::item`] and ends with [`Streaming::end`].
///
/// [`stream`] and [`stream_to`] open it. A stream that is dropped without
/// `end` has printed every line that it was given, and records no exit code.
///
/// A stream ends with exit code 0, 1, or the code that the tool's guide
/// declares for its first failure, as [`Streaming::end`] describes.
#[derive(Debug)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each bool is one independent flag of the run or mark of the stream, and the writer's state is already an enum"
)]
pub struct Streaming<'a, R, O, W> {
    /// Receives the header, the items and, with `--json`, the failures.
    out: O,
    /// Receives the text form of the failures and the report of a failed
    /// write.
    err: W,
    /// Whether the caller asked for JSON with `--json`.
    json: bool,
    /// Whether `--full` asked for every field of each item.
    full: bool,
    /// Whether the run is a dry run.
    dry_run: bool,
    /// The names that `--fields` gave, which an unwrapped document selects.
    fields: &'a [String],
    /// The columns of each item, in order.
    columns: Vec<&'a str>,
    /// The width of each column, which is the width of its header.
    widths: Vec<usize>,
    /// The command tree that a failure's `Run` recovery renders against.
    command: Option<&'a mut Command>,
    /// The spelling of the failures' codes.
    spelling: Spelling,
    /// The tool's own form of a failure, or `None` for crumbkit's form.
    form: Option<&'static dyn FailureForm>,
    /// The stream's document.
    document: Document<Stream<R>>,
    /// Whether the stream printed a failure.
    failed: bool,
    /// The exit codes that the tool's guide declares (ADR-0037).
    exits: &'static [Exit],
    /// The exit code of the stream's first failure, which is 1 until a
    /// failure prints.
    exit: u8,
    /// Whether the output writer still takes lines.
    state: State,
    /// The line being written, which is reused for each line.
    line: Vec<u8>,
    /// The most bytes that the stream prints on the output writer.
    budget: usize,
    /// The bytes written to the output writer, which the budget limits and
    /// the usage log records.
    sent: usize,
    /// The number of items printed.
    items: usize,
    /// The `crumbkit.finish` span, which records the exit code (ADR-0030).
    #[cfg(feature = "tracing")]
    span: tracing::Span,
}

/// Whether the output writer of a stream still takes lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    /// Every write so far succeeded.
    Open,
    /// The reader closed the pipe, so the stream stops and exits with 0.
    Closed,
    /// A write failed for another reason, which the stream reported once, so
    /// the stream stops and exits with 1.
    Broken,
    /// The next item would have passed the output budget, which the stream
    /// reported once, so the stream stops and exits with 1.
    Spent,
}

/// Opens the stream on `out` and `err`, as [`stream_to`] describes.
fn open<R: Row, O: Write, W: Write>(
    out: O,
    err: W,
    output: &mut Output,
    spelling: Spelling,
    document: Document<Stream<R>>,
) -> Result<Streaming<'_, R, O, W>, ExitCode> {
    let budget = output.budget();
    let Output {
        json,
        fields,
        long,
        full,
        command,
        dry_run,
        failure_form,
        exits,
        ..
    } = output;
    let columns: Vec<&str> = if fields.is_empty() {
        let long = if *long { R::LONG_COLUMNS } else { &[] };
        R::COLUMNS.iter().chain(long).copied().collect()
    } else {
        fields.iter().map(String::as_str).collect()
    };
    let mut streaming = Streaming {
        out,
        err,
        json: *json,
        full: *full,
        dry_run: *dry_run,
        fields: fields.as_slice(),
        widths: columns.iter().map(|name| name.chars().count()).collect(),
        columns,
        command: command.as_mut(),
        spelling,
        form: *failure_form,
        document,
        failed: false,
        exits,
        exit: 1,
        state: State::Open,
        line: Vec::new(),
        budget,
        sent: 0,
        items: 0,
        #[cfg(feature = "tracing")]
        span: crate::trace::spans::finish(),
    };
    if let Some(refusal) = fields::refusal::<R>(streaming.fields) {
        return Err(streaming.refuse(&refusal));
    }
    if !streaming.json {
        let header: Vec<Cow<'_, str>> = streaming
            .columns
            .iter()
            .map(|name| Cow::Owned(name.to_uppercase()))
            .collect();
        let written = table::write_line(&mut streaming.line, &header, &streaming.widths);
        // The header is written to a `Vec`, which never fails.
        if written.is_ok() {
            let _ = streaming.send();
        }
    }
    Ok(streaming)
}

impl<R: Row, O: Write, W: Write> Streaming<'_, R, O, W> {
    /// Prints `item` on its own line and flushes the output writer.
    ///
    /// It returns `ControlFlow::Break` when the stream can print nothing
    /// more, because the reader closed the pipe, a write failed or the
    /// output budget is spent, and the tool stops producing items and calls
    /// [`Streaming::end`]. A failed write other than a closed pipe prints
    /// `error: cannot write to standard output:` and its cause on the error
    /// writer once, and the stream ends with exit code 1. An item after the
    /// first whose line would pass the output budget does not print, and the
    /// stream prints the over-budget failure in its place and ends with exit
    /// code 1.
    pub fn item(&mut self, item: &R) -> ControlFlow<()> {
        if self.state != State::Open {
            return ControlFlow::Break(());
        }
        if let Err(error) = self.render(item) {
            self.line.clear();
            return self.stop(&error);
        }
        if self.items > 0 && self.sent.saturating_add(self.line.len()) > self.budget {
            self.line.clear();
            let code = self
                .document
                .over_budget_code()
                .unwrap_or(&codes::OUTPUT_OVER_BUDGET);
            let over = OverBudget {
                code,
                budget: self.budget,
                over: Over::Stream,
            };
            self.report(&over);
            if self.state == State::Open {
                self.state = State::Spent;
            }
            return ControlFlow::Break(());
        }
        self.items += 1;
        self.send()
    }

    /// Returns the bytes that the stream can still print on the output
    /// writer: the budget that [`Output::budget`] had left when the stream
    /// opened, less the bytes printed since. A tool reads it to stop
    /// producing items that cannot print.
    #[must_use]
    pub const fn budget(&self) -> usize {
        self.budget.saturating_sub(self.sent)
    }

    /// Prints `failure` while the stream goes on, and makes the stream end
    /// with exit code 1.
    ///
    /// In text, the failure prints on the error writer in the form that a
    /// failed command prints. With `--json`, it prints on the output writer
    /// as one `crumbkit/failure/v1` document on its own line, between the
    /// items. A document without the envelope prints it as text. When the
    /// tool's guide sets a [`FailureForm`](crate::form::FailureForm), the
    /// form writes the line and the text in place of crumbkit's.
    ///
    /// It returns `ControlFlow::Break` when the stream can print nothing
    /// more, as [`Streaming::item`] does.
    pub fn failure<F: AsFailure>(&mut self, failure: &F) -> ControlFlow<()> {
        self.report(failure.as_failure());
        if self.state == State::Open {
            ControlFlow::Continue(())
        } else {
            ControlFlow::Break(())
        }
    }

    /// Ends the stream with `result`, and returns the process's exit code.
    ///
    /// An `Err` prints as [`Streaming::failure`] prints a failure, and is the
    /// stream's last line. A stream that printed no items and no failure, and
    /// whose pipe is still open, prints an empty note such as `note: no rows`
    /// on the error writer (ADR-0050). A dry run then prints the line `This
    /// was a dry run, so nothing changed.` on the error writer, in text and
    /// with `--json` for a document without the envelope. Each line goes to
    /// the error writer after the output writer was flushed.
    ///
    /// The exit code is 0 when every item printed, and also when the reader
    /// closed the pipe early. When the stream printed a failure or `result`
    /// is an `Err`, the exit code is the one that the tool's guide declares
    /// for the code of the stream's first failure with
    /// [`Exit::failures`](crate::help::Exit::failures), or 1 when it
    /// declares none, as [`finish_to`](crate::finish_to) exits (ADR-0037).
    /// It is 1 when a write failed for a reason other than a closed pipe,
    /// and when the stream stopped at the output budget, unless an earlier
    /// failure chose the code.
    #[must_use]
    #[expect(
        clippy::needless_pass_by_value,
        reason = "end takes the result by value, as finish does, so a tool passes the result of its work in the same way to either"
    )]
    pub fn end<E: AsFailure>(mut self, result: Result<(), E>) -> ExitCode {
        if let Err(error) = &result {
            self.report(error.as_failure());
        }
        let empty = !self.failed && self.items == 0 && self.state == State::Open;
        if empty {
            self.note_empty();
        }
        if self.dry_run
            && self.state == State::Open
            && (!self.json || !self.document.has_envelope())
        {
            self.note(DRY_RUN_LINE);
        }
        let exit = if self.failed {
            self.exit
        } else {
            u8::from(self.state == State::Broken)
        };
        self.record(exit, empty);
        ExitCode::from(exit)
    }

    /// Prints the note of a stream that ended with no rows, such as `note:
    /// no rows`, in the document's own wording when it has one, on the
    /// error writer (ADR-0050).
    fn note_empty(&mut self) {
        let wording = self.document.empty_note().unwrap_or("no rows");
        self.note(&format!("note: {wording}"));
    }

    /// Prints `text` and a line feed on the error writer in one write, and
    /// flushes it. The output writer was flushed after the last line, so a
    /// reader that merges the two streams sees the note after the rows. A
    /// failed write to standard error goes unreported, because it is the
    /// last place to report anything.
    fn note(&mut self, text: &str) {
        self.line.clear();
        self.line.extend_from_slice(text.as_bytes());
        self.line.push(b'\n');
        let _ = self
            .err
            .write_all(&self.line)
            .and_then(|()| self.err.flush());
        self.line.clear();
    }

    /// Writes `item` to the line, in the format that the flags choose.
    fn render(&mut self, item: &R) -> io::Result<()> {
        let line = &mut self.line;
        if self.json && !self.document.has_envelope() {
            return unwrapped::write_json(line, self.fields, item);
        }
        if self.json && self.full {
            // Serializing the item directly keeps every field in its
            // declaration order.
            return write_document(
                line,
                self.document,
                self.dry_run,
                None,
                item,
                &Extras::none(),
            );
        }
        let row = serde_json::to_value(item)?;
        if self.json {
            let selected = Selected {
                row: &row,
                columns: &self.columns,
            };
            write_document(
                line,
                self.document,
                self.dry_run,
                None,
                &selected,
                &Extras::none(),
            )
        } else {
            let cells: Vec<Cow<'_, str>> = self
                .columns
                .iter()
                .map(|name| table::cell(row.get(name)))
                .collect();
            table::write_line(line, &cells, &self.widths)
        }
    }

    /// Prints `failure`, as [`Streaming::failure`] describes, and marks the
    /// stream as failed.
    fn report(&mut self, failure: &dyn Failure) {
        #[cfg(feature = "tracing")]
        let span = self.span.clone();
        #[cfg(feature = "tracing")]
        let _entered = span.enter();
        let document = PreparedFailure::new(
            failure,
            self.spelling,
            self.command.as_deref_mut(),
            &mut self.err,
        );
        // The stream's first failure chooses the exit code, and the span
        // records only that failure, as the usage log does.
        if !self.failed {
            #[cfg(feature = "tracing")]
            crate::trace::spans::failed(document.code());
            self.exit = exit::failed(self.exits, failure.code());
        }
        self.failed = true;
        if self.json && self.document.has_envelope() {
            // A failure document written to a `Vec` never fails, and a closed
            // stream prints nothing more.
            if self.state == State::Open
                && failure::write_json(&mut self.line, self.form, None, &document).is_ok()
            {
                let _ = self.send();
            }
            self.line.clear();
        } else {
            // Standard error is the last place to report anything, so a
            // failure to write there goes unreported.
            let _ = failure::write_text(&mut self.err, self.form, &document);
        }
    }

    /// Refuses the stream with `refusal`, and returns exit code 2.
    fn refuse(mut self, refusal: &crate::refusal::Refusal) -> ExitCode {
        #[cfg(feature = "tracing")]
        let span = self.span.clone();
        #[cfg(feature = "tracing")]
        let _entered = span.enter();
        let code = refusal.print(
            &mut Sink::new(&mut self.line, &mut self.err),
            self.json,
            self.spelling,
            self.form,
            None,
            self.command.as_deref_mut(),
        );
        let _ = self.send();
        self.record(exit::number(code), false);
        code
    }

    /// Writes the line to the output writer, flushes it and clears the line.
    /// It returns `ControlFlow::Break` when the write fails, as
    /// [`Streaming::item`] describes.
    fn send(&mut self) -> ControlFlow<()> {
        let written = self
            .out
            .write_all(&self.line)
            .and_then(|()| self.out.flush());
        if written.is_ok() {
            self.sent = self.sent.saturating_add(self.line.len());
        }
        self.line.clear();
        match written {
            Ok(()) => ControlFlow::Continue(()),
            Err(error) => self.stop(&error),
        }
    }

    /// Stops the stream after `error`. A closed pipe stops it quietly, and
    /// any other error prints once on the error writer.
    fn stop(&mut self, error: &io::Error) -> ControlFlow<()> {
        if error.kind() == io::ErrorKind::BrokenPipe {
            self.state = State::Closed;
        } else {
            // The process is already failing, and a failed report has nowhere
            // else to go.
            let _ = writeln!(self.err, "error: cannot write to standard output: {error}");
            self.state = State::Broken;
        }
        ControlFlow::Break(())
    }

    /// Records the exit code `exit` on the span, and records `exit` and
    /// whether the stream was `empty` in the usage log.
    #[cfg_attr(
        not(any(feature = "tracing", feature = "usage-log")),
        expect(
            clippy::unused_self,
            reason = "only the tracing and usage-log features record the exit code"
        )
    )]
    fn record(&self, exit: u8, empty: bool) {
        #[cfg(feature = "tracing")]
        crate::trace::spans::exited(&self.span, exit);
        #[cfg(feature = "usage-log")]
        crate::usage::finished(
            exit,
            u64::try_from(self.sent).unwrap_or(u64::MAX),
            self.state == State::Spent,
            empty,
        );
        let _ = (exit, empty);
    }
}
