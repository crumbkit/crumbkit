//! The output flags, and `finish`, which prints a result or a failure in the
//! format they choose.

mod budget;
#[cfg(feature = "hand-over")]
mod child;
mod confirm;
mod dry_run;
mod envelope;
pub(crate) mod failure;
mod fields;
mod layout;
mod listing;
mod prose;
mod respond;
mod result;
mod stream;
pub(crate) mod table;
mod terminal;
mod unwrapped;

use std::ffi::OsString;
use std::io::Write;
use std::process::ExitCode;

use clap::{Args, Command};

pub(crate) use self::budget::SMALLEST as SMALLEST_BUDGET;
pub use self::confirm::Yes;
pub use self::dry_run::DryRun;
pub use self::layout::JsonLayout;
pub use self::stream::{Streaming, stream, stream_to};
use crate::form::FailureForm;
use crate::help::Guide;
use crate::{AsFailure, Document, Parsed, Rows, Sink, Spelling};

#[cfg(test)]
mod tests;

/// The flags that choose how a command prints its result.
///
/// A tool adds them to its own clap parser with `#[command(flatten)]`. Every
/// flag is global, so it also works after a subcommand: `shelf books list --json`.
///
/// - `--json` prints a JSON document, on one line unless the guide sets a
///   [`JsonLayout`].
/// - `--long` adds the row type's [`Row::LONG_COLUMNS`](crate::Row::LONG_COLUMNS).
/// - `--full` prints every field of every row in declaration order, and needs `--json`.
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
    /// Print the result as a JSON document
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

    /// Whether the caller passed `--no-input`, which `set_command` copies
    /// from the `Parsed`.
    #[arg(skip)]
    no_input: bool,

    /// The tool's own form of a failure, which `set_command` copies from the
    /// `Parsed`, or `None` for crumbkit's form (ADR-0045).
    #[arg(skip)]
    failure_form: Option<&'static dyn FailureForm>,

    /// Whether the tool's guide turned terminal detection off, which
    /// `set_command` copies from the `Parsed` (ADR-0049).
    #[arg(skip)]
    no_terminal: bool,

    /// The layout of a whole JSON document, which `set_command` copies from
    /// the `Parsed`, or `None` for one line (ADR-0052).
    #[arg(skip)]
    json_layout: Option<JsonLayout>,

    /// The most bytes that the run prints on standard output, which
    /// `set_command` sets from the `Parsed` and the environment, or `None`
    /// for crumbkit's default lowered by the environment (ADR-0046).
    #[arg(skip)]
    budget: Option<usize>,

    /// The bytes that the run has printed on standard output so far.
    #[arg(skip)]
    spent: usize,
}

impl Output {
    /// The output budget of a tool whose guide sets none: 1 MiB, the most
    /// bytes that one run prints on standard output (ADR-0046).
    pub const DEFAULT_BUDGET: usize = budget::DEFAULT;

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
        self.no_input = parsed.no_input;
        self.failure_form = parsed.failure_form;
        self.no_terminal = parsed.no_terminal;
        self.json_layout = parsed.json_layout;
        self.budget = Some(budget::from_environment(
            parsed.output_budget.unwrap_or(budget::DEFAULT),
        ));
        self.command = Some(parsed.command);
    }

    /// Returns the bytes that the run can still print on standard output:
    /// the output budget less what `finish` or a stream has printed.
    ///
    /// The budget is the one that the tool's
    /// [`Guide::output_budget`](crate::help::Guide::output_budget) sets, or
    /// [`Output::DEFAULT_BUDGET`], lowered by the variable
    /// `CRUMBKIT_OUTPUT_BUDGET` when it holds a smaller whole number of
    /// bytes. A handler reads it before `finish` to fetch no more rows than
    /// can print. `finish` prints a result within it, as [`finish_to`]
    /// describes (ADR-0046).
    #[must_use]
    pub fn budget(&self) -> usize {
        let budget = self
            .budget
            .unwrap_or_else(|| budget::from_environment(budget::DEFAULT));
        budget.saturating_sub(self.spent)
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

    /// Returns the output of a run that `parse` ended after it printed help,
    /// the version or a refusal: `--json` as the walk found it, the command
    /// tree that `parse` built, and the guide's failure form and terminal
    /// detection.
    pub(crate) fn stopped(json: bool, command: Command, guide: &Guide) -> Self {
        Self {
            json,
            command: Some(command),
            failure_form: guide.get_failure_form(),
            no_terminal: !guide.terminal_detection(),
            json_layout: guide.layout(),
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
/// JSON format is one line holding an object, or the object indented by two
/// spaces when the guide sets [`JsonLayout::Pretty`] (ADR-0052),
/// with the `schema` id of `document` and the result under `data`; a single
/// row's `data` is an object, and a `Vec`'s is an array.
/// A document from [`Document::without_envelope`] prints the result alone,
/// and its failures print as text on the error writer even with `--json`.
///
/// A [`Listing`](crate::Listing)'s `data` is an array, and its JSON document
/// also holds the number of matching rows under `total` when the tool
/// counted them, `truncated`, which is true when the document leaves out
/// rows, and its summary, when it has one, under `summary`. The summary also prints as one line
/// above the table in text.
///
/// In text, the lines that describe a listing print on the error writer
/// after the output writer is flushed, so the output writer holds the
/// result alone (ADR-0050). When a listing shows fewer rows than its total,
/// a footer line says so, such as `Showing 4 of 6 books. --all shows every
/// one.`, and a listing without a total that leaves out rows prints
/// `Showing 4 books, and more match. --all shows every one.` A listing whose total is 0 prints an empty note, such as `note: no
/// books match`. A listing's note follows on its own line. When a listing
/// has a next step, a `Next:` line follows, such as
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
/// A [`Bytes`](crate::Bytes) prints its bytes unchanged, with no escaping
/// and no added line feed, and its JSON document holds `{"text": "..."}`
/// under `data` when the bytes are valid UTF-8 and `{"base64": "..."}`
/// otherwise. `--fields`, `--long` and `--full` are refused with exit code
/// 2, because it has no columns.
///
/// On a dry run, the line `This was a dry run, so nothing changed.` prints
/// last on the error writer, after a batch's failures, in text and with
/// `--json` for a document without the envelope.
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
/// output writer as one `crumbkit/failure/v1` document, on one line or in
/// the guide's layout, which `--full` and `--fields` do not change.
///
/// When the tool's guide sets a
/// [`FailureForm`](crate::form::FailureForm), the form writes the JSON
/// document and the text of the failure and of a refusal in place of
/// crumbkit's, and the text of each failure of a batch or a prose result.
/// Each entry of a batch's `failures` keeps the shape described above
/// (ADR-0045).
///
/// The result prints within [`Output::budget`] bytes on the output writer.
/// `finish` measures it first, and prints a result that fits as described
/// above. A listing or a `Vec` that does not fit prints the most rows that
/// fit and exits 0: in text, a line on the error writer such as `Showing
/// 812 of 5000 books, because output stops at 1048576 bytes. Ask for less
/// with --limit or --fields.` says so, and its JSON document holds `cut` with the code
/// `crumbkit::output::over_budget` and the budget. A batch prints the rows
/// that fit and ends with one over-budget failure whose fields are
/// `left_out` and `budget`, and exits 1. A single row, a prose result or a
/// bytes result that does not fit prints nothing but the over-budget
/// failure, and exits 1, unless its document is marked with
/// `Document::always_print_first` (ADR-0046). A bytes result is measured in
/// the form it prints, so with `--json` its base64 text counts.
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
/// names an unknown field, or when a bytes result gets `--fields`, `--long`
/// or `--full`, which are checked before the result prints.
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
    let truncated = result.as_ref().is_ok_and(listing::leaves_out);
    #[cfg(feature = "usage-log")]
    let empty = result.as_ref().is_ok_and(|value| value.total() == Some(0));
    #[cfg(feature = "usage-log")]
    let sink = &mut Sink::new(crate::usage::Counted::new(&mut sink.out), &mut sink.err);
    // With the `tracing` feature, the response runs inside the
    // `crumbkit.finish` span, which records the exit code (ADR-0030).
    #[cfg(feature = "tracing")]
    let span = crate::trace::spans::finish();
    #[cfg(feature = "tracing")]
    let _entered = span.enter();
    let (code, cut) = respond::respond(sink, output, spelling, document, result);
    #[cfg(feature = "tracing")]
    crate::trace::spans::exited(&span, code);
    #[cfg(feature = "usage-log")]
    crate::usage::finished(code, sink.out.bytes(), truncated || cut, empty);
    #[cfg(not(feature = "usage-log"))]
    let _ = cut;
    code
}
