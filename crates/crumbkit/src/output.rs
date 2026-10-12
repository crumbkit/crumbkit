//! The output flags, and `finish`, which prints a result or a failure in the
//! format they choose.

mod confirm;
mod dry_run;
mod envelope;
pub(crate) mod failure;
mod fields;
mod listing;
mod prose;
mod stream;
mod table;
mod unwrapped;

use std::ffi::OsString;
use std::io::{self, Write};
use std::process::ExitCode;

use clap::{Args, Command};
use serde_json::Value;

pub use self::confirm::Yes;
use self::dry_run::DRY_RUN_LINE;
pub use self::dry_run::DryRun;
use self::envelope::{Selected, write_document};
use self::failure::FailureDocument;
pub use self::stream::{Streaming, stream, stream_to};
use crate::{AsFailure, Document, Parsed, Row, Rows, Shape, Sink, Spelling};

#[cfg(test)]
mod tests;

/// The flags that choose how a command prints its result.
///
/// A tool adds them to its own clap parser with `#[command(flatten)]`. Every
/// flag is global, so it also works after a subcommand, as in
/// `shelf books list --json`.
///
/// - `--json` prints a JSON document on one line.
/// - `--long` adds the row type's [`Row::LONG_COLUMNS`].
/// - `--full` prints every field of every row in declaration order, and
///   requires `--json`.
/// - `--fields a,b` prints exactly the named fields in that order, and
///   conflicts with `--long` and `--full`.
///
/// It also holds the command tree that the tool parsed against, which
/// [`Output::set_command`] sets, so `finish` can render a listing's next step
/// and a failure's recovery. `finish` builds only the branch of the tree that
/// such a call names.
///
/// `Output` derives `Clone`, `Debug` and `Default`. It does not derive
/// `PartialEq` or `Eq`, because `clap::Command` implements neither and nothing
/// compares two `Output` values.
#[derive(Args, Clone, Debug, Default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each bool is one independent flag or mark of the run, and no two of them form a state machine"
)]
pub struct Output {
    /// Print the result as a JSON document on one line
    #[arg(long, global = true)]
    json: bool,

    /// Print only these comma-separated fields, in this order
    #[arg(
        long,
        global = true,
        value_name = "FIELD",
        value_delimiter = ',',
        conflicts_with_all = ["long", "full"]
    )]
    fields: Vec<String>,

    /// Add the long columns after the compact ones
    #[arg(long, global = true)]
    long: bool,

    /// With --json, print every field of every row
    #[arg(long, global = true, requires = "json")]
    full: bool,

    /// The command tree that the tool parsed against, or `None` until the
    /// tool calls `set_command`. Its root and the commands on the typed path
    /// are built, and the rest is built when a reader needs it. clap skips
    /// the field when it parses.
    #[arg(skip)]
    command: Option<Command>,

    /// The words that the caller typed after the program name, which
    /// `confirm` repeats with `--yes`, or `None` when the tool passed a bare
    /// command to `set_command`.
    #[arg(skip)]
    words: Option<Vec<OsString>>,

    /// Whether the run is a dry run, which [`Output::dry_run`] records.
    #[arg(skip)]
    dry_run: bool,
}

impl Output {
    /// Keeps `command` so that [`finish`] can render a listing's next step
    /// and a failure's [`Recovery::Run`](crate::Recovery::Run) call against
    /// it. A tool's `main` calls it once after parsing, with the command
    /// tree that it parsed against, which [`parse`](crate::parse) returns.
    ///
    /// It does not build the tree. Parsing has already built the root and
    /// each command on the typed path, `finish` builds only the branch that
    /// a call names, and [`Output::command`] builds the whole tree for the
    /// commands that read it, so a run builds only what it reads. Without a
    /// command, `finish` leaves out the `Next:` line and prints a failure's
    /// `Run` recovery as having no next step.
    ///
    /// A call into a branch below the root resolves the root's global
    /// arguments, such as `--json`, only when the root was built by parsing.
    /// A tree that never went through parsing, such as a fresh
    /// `Cli::command()` beside clap's `Cli::parse`, leaves out a `Next:` line
    /// or a recovery whose call names one.
    ///
    /// A tool that parses with crumbkit passes the command that
    /// [`parse`](crate::parse) returns:
    ///
    /// ```
    /// use clap::{CommandFactory, Parser};
    /// use crumbkit::help::Guide;
    /// use crumbkit::{Output, Spelling};
    ///
    /// /// List the books in a small catalogue.
    /// #[derive(Parser)]
    /// struct Cli {
    ///     #[command(flatten)]
    ///     output: Output,
    /// }
    ///
    /// let parsed =
    ///     crumbkit::parse_from::<Cli, _, _>(Cli::command(), &Guide::new(), Spelling::COLONS, ["shelf"]);
    /// let (mut cli, parsed) = parsed.unwrap();
    /// cli.output.set_command(parsed);
    /// ```
    ///
    /// A tool that hides commands at runtime parses against its changed
    /// command and passes that same command:
    ///
    /// ```
    /// use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
    /// use crumbkit::Output;
    ///
    /// /// Browse a small catalogue.
    /// #[derive(Parser)]
    /// struct Cli {
    ///     #[command(subcommand)]
    ///     command: Commands,
    ///     #[command(flatten)]
    ///     output: Output,
    /// }
    ///
    /// #[derive(Subcommand)]
    /// enum Commands {
    ///     /// List the books.
    ///     List,
    ///     /// Remove a book.
    ///     Remove,
    /// }
    ///
    /// let mut command =
    ///     Cli::command().mut_subcommand("remove", |remove| remove.hide(true));
    /// let matches = command.try_get_matches_from_mut(["shelf", "list"]).unwrap();
    /// let mut cli = Cli::from_arg_matches(&matches).unwrap();
    /// cli.output.set_command(command);
    /// ```
    pub fn set_command(&mut self, parsed: impl Into<Parsed>) {
        let parsed = parsed.into();
        self.words = parsed.words;
        self.command = Some(parsed.command);
    }

    /// Builds the whole command tree that [`Output::set_command`] kept and
    /// returns it, or returns `None` before the tool calls `set_command`.
    ///
    /// `commands::list`, `schemas::list`, `schemas::show`, the usage reports
    /// and crumbkit-aliases' `list` read the tree through it. It calls
    /// `Command::build`, which clap requires before a command's arguments
    /// are read, and which does nothing to a command already built, so the
    /// tree holds each command's global arguments from above, its help and
    /// version flags and its argument groups. It takes `&mut self` because
    /// it builds the tree in place, which is never cloned at runtime.
    pub fn command(&mut self) -> Option<&Command> {
        let command = self.command.as_mut()?;
        command.build();
        Some(command)
    }

    /// Reports whether the caller asked for JSON with `--json`.
    ///
    /// It answers on every path of a run. After parsing, it reads the flag
    /// that clap parsed. After [`parse`](crate::parse) printed help, the
    /// version or a refusal, the [`Stop`](crate::Stop) that it returned holds
    /// an `Output` whose answer comes from the walk of the typed words, which
    /// counts `--json` only where clap reads it as a flag, and never where it
    /// is the value of an option. A refusal prints as a JSON document exactly
    /// when this is true.
    #[must_use]
    pub const fn is_json(&self) -> bool {
        self.json
    }

    /// Reports whether the tool may draw progress, such as a spinner or a
    /// bar, on standard error. It is true only when `--json` is off and
    /// standard error is a terminal.
    ///
    /// A tool draws progress with any library it likes when this is true,
    /// and prints nothing extra otherwise, so a program or an agent that
    /// reads the output, and a pipe or a file, receive no progress lines.
    /// `check_examples` in crumbkit-test runs each example with standard
    /// error piped, so an example never draws progress.
    #[must_use]
    pub fn progress_allowed(&self) -> bool {
        !self.json && crate::sink::stderr_is_terminal()
    }

    /// Returns the output of a run that `parse` ended after it printed help,
    /// the version or a refusal: `--json` as the walk found it, and the
    /// command tree that `parse` built.
    pub(crate) fn stopped(json: bool, command: Command) -> Self {
        Self {
            json,
            command: Some(command),
            ..Self::default()
        }
    }
}

/// Prints a command's result in the format that `output` chooses, and
/// returns the process's exit code.
///
/// A tool's `main` calls it last with the handler's `result` and returns its
/// exit code. It behaves as [`finish_to`] does with a sink over the process's
/// standard output and standard error.
pub fn finish<T: Rows, E: AsFailure>(
    output: &mut Output,
    spelling: Spelling,
    document: Document<T>,
    result: Result<T, E>,
) -> ExitCode {
    finish_to(&mut Sink::standard(), output, spelling, document, result)
}

/// Prints a command's result to `sink` in the format that `output` chooses,
/// and returns the exit code that [`finish`] would return.
///
/// Tests and golden files call it with a sink over in-memory writers, so they
/// check a command's exact output and exit code without starting a process.
///
/// An `Ok` value prints to the sink's output writer. The text format is a
/// table with a header row, and an empty result prints the header alone. The
/// JSON format is one line holding an object
/// with the `schema` id of `document` and the result under `data`; a single
/// row's `data` is an object, and a `Vec`'s is an array.
/// A document from [`Document::without_envelope`] prints the result alone,
/// and its failures print as text on the error writer even with `--json`.
///
/// A [`Listing`](crate::Listing)'s `data` is an array, and its JSON document
/// also holds the number of matching rows under `total`, and its summary,
/// when it has one, under `summary`. The summary also prints as one line
/// above the table in text. When a listing
/// shows fewer rows than its total, the table ends with a footer line, such
/// as `Showing 4 of 6 books. --all shows every one.` A listing's note
/// follows on its own line, in text only. When a listing has a next step, a
/// `Next:` line follows the table, any footer and any note, such as
/// `Next: shelf books show <ID>... takes up to 100 ids in one call.` The line
/// is left out when [`Output::set_command`] was not called or the step's
/// [`Call`](crate::Call) does not resolve in that command, and the exit code
/// stays 0.
///
/// `finish` takes `output` by `&mut` because it builds, in place, the branch
/// of the command tree that a next step's call or a failure's `Run` call
/// names, and nothing else (ADR-0015). A handler that reads `output` with
/// `&mut`, such as [`commands::list`](crate::commands::list), runs before
/// `finish`, and `main` passes its result in.
///
/// A [`Batch`](crate::Batch)'s rows print as a `Vec`'s do, and its JSON
/// document also holds its failures under `failures`, which is present even
/// when the list is empty. Each entry of `failures` has the fields of the
/// `crumbkit/failure/v1` document described below, without `schema`. In
/// text, the rows go to the output writer, which is flushed, and then each
/// failure goes to the error writer in the form a failed command prints, in
/// order. A batch with no rows writes nothing to the output writer in text,
/// so the caller sees only the failures. `--long`, `--full` and `--fields`
/// shape the rows and leave the failures unchanged.
///
/// A [`Prose`](crate::Prose) prints its text, with each control character
/// other than a line feed or a tab escaped and a line feed added when the
/// text lacks one, and its JSON document holds its value under `data`.
/// Its failures print as a batch's do. `--long` and `--full` leave it
/// unchanged, and `--fields` is refused with exit code 2, because it has no
/// columns.
///
/// An `Err` prints the failure. The text form goes to the sink's error
/// writer, as shelf's golden file [`books-show-7.stderr`][books-show-7]
/// shows, and nothing goes to the output writer.
///
/// The `error:` line is the failure's `Display`. A `cause:` line follows it
/// for each error in the failure's `source()` chain, and a `url:` line
/// follows the `code:` line when the failure has a url. The code prints in
/// `spelling`. A [`Recovery::Run`](crate::Recovery::Run) call is rendered
/// against the command that [`Output::set_command`] received, and prints as
/// `no next step: the suggested command is not available` when no command is
/// set or the call does not resolve. With `--json`, the failure prints to the
/// output writer as one `crumbkit/failure/v1` document on one line, which
/// `--full` and `--fields` do not change.
///
/// When `--fields` names a field that the row type does not have, nothing of
/// the result prints. The refusal prints as a failure does, with the message
/// `--fields names an unknown field 'radius'`, the code `crumbkit::cli::usage`
/// in `spelling`, and the hint `use the valid fields: a, b`.
///
/// The exit code is 0 when the result is written, and also when the reader
/// closed the pipe early. It is 1 when the result is a failure or a batch
/// with at least one failure, also after the reader closed the pipe early.
/// It is 1 as well when a write fails for any other reason, or when the
/// result cannot be serialized, and the cause of a failed write goes to the
/// sink's error writer before a batch's failures. It is 2 when `--fields`
/// names an unknown field, which is checked before the result prints.
///
/// [books-show-7]: https://github.com/crumbkit/crumbkit/blob/main/examples/shelf/tests/golden/books-show-7.stderr
pub fn finish_to<T: Rows, E: AsFailure, O: Write, W: Write>(
    sink: &mut Sink<O, W>,
    output: &mut Output,
    spelling: Spelling,
    document: Document<T>,
    result: Result<T, E>,
) -> ExitCode {
    // With the usage log, the sink counts the bytes of standard output, and
    // the run's line records them with the exit code (ADR-0029).
    #[cfg(feature = "usage-log")]
    let truncated = result.as_ref().is_ok_and(|value| {
        value
            .total()
            .is_some_and(|total| value.rows().len() < total)
    });
    #[cfg(feature = "usage-log")]
    let sink = &mut Sink::new(crate::usage::Counted::new(&mut sink.out), &mut sink.err);
    // With the `tracing` feature, the response runs inside the
    // `crumbkit.finish` span, which records the exit code (ADR-0030).
    #[cfg(feature = "tracing")]
    let span = crate::trace::spans::finish();
    #[cfg(feature = "tracing")]
    let _entered = span.enter();
    let code = respond(sink, output, spelling, document, result);
    #[cfg(feature = "tracing")]
    crate::trace::spans::exited(&span, code);
    #[cfg(feature = "usage-log")]
    crate::usage::finished(code, sink.out.bytes(), truncated);
    code
}

/// Prints the result to `sink` and returns the exit code, as [`finish_to`]
/// describes.
#[expect(
    clippy::needless_pass_by_value,
    reason = "finish_to takes the result by value, as finish does, so a tool passes the result it built in the same way to either"
)]
fn respond<T: Rows, E: AsFailure, O: Write, W: Write, R>(
    sink: &mut Sink<O, W, R>,
    output: &mut Output,
    spelling: Spelling,
    document: Document<T>,
    result: Result<T, E>,
) -> ExitCode {
    if let Some(refusal) = fields::refusal::<T>(&output.fields) {
        return refusal.print(sink, output.json, spelling, output.command.as_mut());
    }
    let value = match &result {
        Ok(value) => value,
        Err(error) => {
            let command = output.command.as_mut();
            failure::print(sink, output.json, spelling, error.as_failure(), command);
            return ExitCode::FAILURE;
        }
    };
    // A batch's failures are read once, so its JSON document and its text
    // form print the same fields. Every other result has `None` here.
    let failures: Option<Vec<FailureDocument>> = value.failures().map(|failures| {
        failures
            .into_iter()
            .map(|failure| FailureDocument::new(failure, spelling, output.command.as_mut()))
            .collect()
    });
    let code = match write_result(&mut sink.out, output, document, value, failures.as_deref()) {
        Ok(()) if output.dry_run && output.json && !document.has_envelope() => {
            // A document without the envelope has no `dry_run` field, so the
            // line that marks a dry run goes to standard error, and a failed
            // write there goes unreported.
            let _ = writeln!(sink.err, "{DRY_RUN_LINE}");
            ExitCode::SUCCESS
        }
        Ok(()) => ExitCode::SUCCESS,
        // A reader that stops early, as `shelf books list | head -1` does,
        // closes the pipe after it has read every line it wanted, so the
        // command did its job.
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => {
            // The process is already failing, and a failed report has nowhere
            // else to go.
            let _ = writeln!(sink.err, "error: cannot write to standard output: {error}");
            ExitCode::FAILURE
        }
    };
    let Some(failures) = failures else {
        return code;
    };
    // The span records only the batch's first failure, as the usage log does.
    #[cfg(feature = "tracing")]
    if let Some(first) = failures.first() {
        crate::trace::spans::failed(first.code());
    }
    // A document without the envelope has no `failures` field, so its
    // failures print as text even with `--json`.
    if !output.json || !document.has_envelope() {
        for failure in &failures {
            // Standard error is the last place to report anything, so a
            // failure to write there goes unreported.
            let _ = failure::write_text(&mut sink.err, failure);
        }
    }
    if failures.is_empty() {
        code
    } else {
        ExitCode::FAILURE
    }
}

/// Writes `value` to `out` in the format that `output` chooses, then flushes
/// `out`. `failures` holds the failures of a batch or a prose result, which
/// the JSON document holds under `failures`, and is `None` for every other
/// result.
///
/// A serialization error becomes an `io::Error` of kind `InvalidData`, so it
/// is reported as a failed write. The text of a dry run ends with
/// [`DRY_RUN_LINE`].
fn write_result<T: Rows>(
    out: &mut impl Write,
    output: &mut Output,
    document: Document<T>,
    value: &T,
    failures: Option<&[FailureDocument]>,
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
    failures: Option<&[FailureDocument]>,
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
