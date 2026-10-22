//! The output flags, and `finish`, which prints a result or a failure in the
//! format they choose.

mod budget;
mod build;
mod children;
mod columns;
mod confirm;
mod direct;
pub(crate) mod dry_run;
mod envelope;
pub(crate) mod failure;
pub(crate) mod fields;
mod layout;
mod lines;
mod listing;
mod problems;
mod prose;
mod respond;
mod result;
mod room;
mod stream;
pub(crate) mod table;
mod terminal;
mod unwrapped;

use std::ffi::OsString;
use std::io::Write;
use std::process::ExitCode;

use clap::Command;

pub use self::build::ViewFlags;
pub use self::confirm::Yes;
pub use self::dry_run::DryRun;
use self::failure::Printing;
pub use self::layout::JsonLayout;
pub use self::room::Room;
pub use self::stream::{Streaming, stream, stream_to};
use crate::help::Exit;
use crate::help::rename::Renames;
use crate::view::View;
use crate::{AsFailure, Document, Rows, Sink, Spelling};

#[cfg(test)]
mod tests;

/// The output of one run: the view flags that the caller typed, the command
/// tree that the tool parsed against, the words that the caller typed and
/// what the tool's guide decides about printing.
///
/// [`parse`](crate::parse()) returns it beside the tool's parsed type, and
/// a tool that parses with clap alone builds it with [`Output::new`]. Both
/// record `--dry-run`, which [`Output::is_dry_run`] reports. A tool's
/// `main` passes it by value to [`finish`] or [`stream`] with the handler's
/// result, as the last call of the run (ADR-0094).
///
/// [`ViewFlags`] describes the view flags. A leaf whose document cannot use
/// `--fields`, `--long` or `--full` leaves that flag out of its help, and
/// `parse` and `finish` refuse it, as
/// [`help::Leaf::offers`](crate::help::Leaf::offers) describes.
///
/// `parse` and [`Output::new`] refuse `--full` without `--json`, and
/// `--fields` with `--long` or `--full`, as a refused command line with the
/// code `<tool>::cli::usage`, such as `--full prints every field of every
/// row in JSON, so it needs --json` with the hint `add --json, or drop
/// --full`. clap parses each of these lines, so a tool that parses with
/// clap alone gets the refusal from `Output::new`, before its handler
/// runs. An `Output` holds only a view that every printer accepts
/// (ADR-0093).
///
/// `finish` renders a listing's next step and a failure's recovery against
/// the command tree, and builds only the branch of the tree that such a
/// call names.
///
/// `Output` derives `Clone` and `Debug`. It does not derive `PartialEq` or
/// `Eq`, because `clap::Command` implements neither and nothing compares
/// two `Output` values.
#[derive(Clone, Debug)]
pub struct Output {
    /// The view that the four view flags selected, which `View::try_from`
    /// built after it refused the pairs that cannot work together.
    view: View,

    /// The command tree that the tool parsed against. Its root and the
    /// commands on the typed path are built, and the rest is built when a
    /// reader needs it.
    command: Command,

    /// The words that the caller typed after the program name, which
    /// `confirm` repeats with `--yes`.
    words: Vec<OsString>,

    /// Whether the run is a dry run: `parse` read `--dry-run` on the
    /// deepest command, or the tool passed the flag to [`Output::new`].
    dry_run: bool,

    /// Whether the caller passed `--no-input`.
    no_input: bool,

    /// Whether the tool's guide turned terminal detection off (ADR-0049).
    no_terminal: bool,

    /// The layout of a whole JSON document (ADR-0052).
    json_layout: JsonLayout,

    /// The most bytes that the run prints on standard output: the guide's
    /// budget or crumbkit's default, lowered by the environment (ADR-0046).
    budget: usize,

    /// The exit codes that the tool's guide declares. A failure whose code
    /// one of them lists ends the run with that exit code (ADR-0037).
    exits: &'static [Exit],

    /// The tool's own codes for crumbkit's failures, of the guide and of
    /// the leaf of the parsed command (ADR-0056, ADR-0060).
    renames: Renames,

    /// The guide's spelling of the run's codes, which writes the tool's
    /// name in crumbkit's own codes. The tool's name also names crumbkit's
    /// documents and variables (ADR-0079).
    spelling: Spelling,
}

impl Output {
    /// The output budget of a tool whose guide sets none: 1 MiB, the most
    /// bytes that one run prints on standard output (ADR-0046).
    pub const DEFAULT_BUDGET: usize = budget::DEFAULT;

    /// Returns the output budget: the most bytes that the run prints on
    /// standard output.
    ///
    /// The budget is the one that the tool's
    /// [`Guide::output_budget`](crate::help::Guide::output_budget) sets, or
    /// [`Output::DEFAULT_BUDGET`], lowered by the variable
    /// `<TOOL>_OUTPUT_BUDGET` when it holds a smaller whole number of
    /// bytes. A handler reads it before `finish` to fetch no more rows than
    /// can print. `finish` prints a result within it, as [`finish_to`]
    /// describes, and a stream prints within it from the first line, as
    /// [`Streaming::budget`] reports (ADR-0046).
    #[must_use]
    pub const fn budget(&self) -> usize {
        self.budget
    }

    /// Builds the whole command tree that the run parsed against and
    /// returns it.
    ///
    /// `commands::list`, `schemas::list`, `schemas::show`, the usage reports
    /// and crumbkit-aliases' `list` read the tree through it. It calls
    /// `Command::build`, which clap requires before a command's arguments
    /// are read, and which does nothing to a command already built, so the
    /// tree holds each command's global arguments from above, its help and
    /// version flags and its argument groups. It takes `&mut self` because
    /// it builds the tree in place, which is never cloned at runtime.
    pub fn command(&mut self) -> &Command {
        self.command.build();
        &self.command
    }

    /// Reports whether the caller asked for JSON with `--json`.
    ///
    /// It answers on every path of a run. After parsing, it reads the flag
    /// that clap parsed. After [`parse`](crate::parse) printed help, the
    /// version or a refusal, the [`Stop`](crate::Stop) that it returned holds
    /// an `Output` whose answer comes from the walk of the typed words, which
    /// counts `--json` only where clap reads it as a flag, and never where it
    /// is the value of an option. The walk also leaves out a `--json` that no
    /// command on the typed path declares, because clap refuses such a word
    /// as an unexpected argument. A refusal prints as a JSON document exactly
    /// when this is true, unless the tool chose its form with
    /// [`Refused::as_json`](crate::Refused::as_json).
    #[must_use]
    pub const fn is_json(&self) -> bool {
        self.view.is_json()
    }

    /// Returns the view that the view flags selected.
    pub(crate) const fn view(&self) -> &View {
        &self.view
    }

    /// Returns the words that the caller typed after the program name.
    #[cfg(test)]
    pub(crate) fn words(&self) -> &[OsString] {
        &self.words
    }

    /// Reports whether the caller passed `--no-input`.
    #[cfg(test)]
    pub(crate) const fn no_input(&self) -> bool {
        self.no_input
    }

    /// Returns the fields that `--fields` named, or an empty slice without
    /// the flag.
    #[cfg(test)]
    pub(crate) fn field_list(&self) -> &[String] {
        self.view.fields()
    }

    /// Returns the guide's spelling and `document`, with the tool's name in
    /// place of `crumbkit` in crumbkit's own codes and documents (ADR-0079).
    pub(crate) const fn named<T>(&self, document: Document<T>) -> (Spelling, Document<T>) {
        (self.spelling, self.spelling.tool().document(document))
    }

    /// Returns the command tree that the tool parsed against, as far as it
    /// is built.
    pub(crate) const fn command_tree(&self) -> &Command {
        &self.command
    }

    /// Returns what the tool's guide decides about the run's failures: the
    /// JSON layout, the exit codes and the renames.
    pub(crate) const fn printing(&self) -> Printing {
        Printing {
            layout: self.json_layout,
            exits: self.exits,
            renames: self.renames,
        }
    }
}

/// Reports whether the caller gave `value`, a batch, more than one key.
/// A headed batch prints its `==> KEY <==` headers by this rule.
fn several_keys<T: Rows>(value: &T) -> bool {
    value.key(1).is_some()
}

/// Prints a command's result in the format that `output` chooses, and
/// returns the process's exit code.
///
/// A tool's `main` calls it last with the handler's `result` and returns its
/// exit code. It behaves as [`finish_to`] does with a sink over the process's
/// standard output and standard error that has a width: when standard
/// output is a terminal and the guide keeps terminal detection on, `finish`
/// gives its sink the terminal's number of columns, so a text table wider
/// than the terminal narrows its columns to fit, and the output budget
/// still counts the unfitted table (ADR-0121). `finish_to` fits a table to
/// the width that its sink has, which [`Sink::fitted`] sets (ADR-0134).
///
/// It takes `output` by value, because a run prints one result, so `main`
/// cannot call it twice with one `Output` (ADR-0094):
///
/// ```compile_fail
/// # #[derive(serde::Serialize, schemars::JsonSchema)]
/// # struct Entry { position: u32 }
/// # impl crumbkit::Row for Entry { const COLUMNS: &'static [&'static str] = &["position"]; }
/// use crumbkit::{Document, Error, Output};
///
/// const ENTRIES: Document<Vec<Entry>> =
///     Document::new("reading", "entries", 1, "The entries of a reading list.");
///
/// let output = Output::new(
///     crumbkit::ViewFlags::new(),
///     crumbkit::DryRun::default(),
///     clap::Command::new("reading"),
///     &crumbkit::help::Guide::new(),
///     ["reading"],
/// )
/// .unwrap();
/// let first: Result<Vec<Entry>, Error> = Ok(Vec::new());
/// let second: Result<Vec<Entry>, Error> = Ok(Vec::new());
/// crumbkit::finish(output, ENTRIES, first);
/// crumbkit::finish(output, ENTRIES, second);
/// ```
pub fn finish<T: Rows, E: AsFailure>(
    output: Output,
    document: Document<T>,
    result: Result<T, E>,
) -> ExitCode {
    // The sink gives the result the width of the terminal that standard
    // output is, which a text table fits (ADR-0121, ADR-0128).
    let sink = &mut Sink::standard().fitted(output.read_terminal_width());
    ExitCode::from(finish_to(sink, output, document, result))
}

/// Prints a command's result to `sink` in the format that `output` chooses,
/// and returns the exit code that [`finish`] would return, as a number.
///
/// Tests and golden files call it with a sink over in-memory writers, so they
/// check a command's exact output and exit code without starting a process.
/// A tool that needs the exit code as a number, such as one that does more
/// work after it prints, calls it with [`Sink::standard`]. `ExitCode` cannot
/// be read back as a number, and a guide can give a failure any code from 3
/// to 125, so the number comes from here.
///
/// An `Ok` value prints to the sink's output writer. The text format is a
/// table with a header row, and an empty result prints the header alone. The
/// JSON format is one line holding an object, or the object indented by two
/// spaces when the guide sets [`JsonLayout::Pretty`] (ADR-0052),
/// with the `schema` id of `document` and the result under `data`; a single
/// row's `data` is an object, and a `Vec`'s is an array.
///
/// When the sink has a width, which [`Sink::fitted`] sets, a text table
/// wider than that width narrows its columns to fit, as `finish` fits it
/// to a terminal, and the output budget still counts the unfitted table. A
/// sink from [`Sink::new`] has no width, so its table prints unfitted
/// (ADR-0121, ADR-0134).
/// A document with [`Envelope::Without`](crate::Envelope::Without) prints
/// the result alone, and its failures print as text on the error writer
/// even with `--json`.
///
/// A [`Listing`](crate::Listing)'s `data` is an array, and its JSON document
/// also holds the number of matching rows under `total` when the tool
/// counted them, `truncated`, which is true when the document leaves out
/// rows, and its summary, when it has one, under `summary`. The summary also prints as one line
/// above the table in text, unless its `Display` writes no text.
///
/// In text, the lines that describe a listing print on the error writer
/// after the output writer is flushed, so the output writer holds the
/// result alone (ADR-0050). When a listing shows fewer rows than its total,
/// a footer line says so, such as `Showing 4 of 6 books. --all shows every
/// one.`, and a listing without a total that leaves out rows prints
/// `Showing 4 books, and more match. --all shows every one.` A listing
/// with no rows and no more rows prints an empty note, such as `note: no
/// books match`. A listing's note follows on its own line. When a listing
/// has a next step, a `Next:` line follows, such as
/// `Next: shelf books show <ID>... takes up to 100 ids in one call.` The line
/// is left out when the step's [`Call`](crate::Call) does not resolve in
/// the command tree that `output` holds, and the exit code stays 0.
///
/// `finish` takes `output` by value, because it is the last call of a run:
/// it builds the branch of the command tree that a next step's call or a
/// failure's `Run` call names, and nothing reads the `Output` after it
/// (ADR-0015, ADR-0094). A handler that reads `output` with `&mut`, such as
/// [`commands::list`](crate::commands::list), runs before `finish`, and
/// `main` passes its result in.
///
/// A [`Batch`](crate::Batch)'s rows print as a `Vec`'s do, and its JSON
/// document also holds its failures under `failures`, which is present even
/// when the list is empty. Each entry of `failures` has the fields of the
/// `<tool>/failure/v1` document described below, without `schema`. In
/// text, the rows go to the output writer, which is flushed, and then each
/// failure goes to the error writer in the form a failed command prints, in
/// order. A batch with no rows writes nothing to the output writer in text,
/// so the caller sees only the failures. `--long`, `--full` and `--fields`
/// shape the rows and leave the failures unchanged.
///
/// A [`Prose`](crate::Prose) prints its text, with each control character
/// other than a line feed or a tab escaped and a line feed added when the
/// text lacks one, and its JSON document holds its value under `data`.
/// Its failures print as a batch's do. `--fields`, `--long` and `--full`
/// are refused, because it has no columns.
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
/// the spelling that `output` took from the guide. A
/// [`Recovery::Run`](crate::Recovery::Run) call is rendered against the
/// command tree that `output` holds, and prints as `no next step: the
/// suggested command is not available` when the call does not resolve. With
/// `--json`, the failure prints to the output writer as one
/// `<tool>/failure/v1` document, on one line or in the guide's layout,
/// which `--full` and `--fields` do not change.
///
/// The result prints within [`Output::budget`] bytes on the output writer.
/// `finish` measures it first, and prints a result that fits as described
/// above. A listing or a `Vec` that does not fit prints the most rows that
/// fit and exits 0: in text, a line on the error writer such as `Showing
/// 812 of 5000 books, because output stops at 1048576 bytes. Ask for less
/// with --limit or --fields.` says so, and its JSON document holds `cut`
/// with the code `<tool>::output::over_budget`, or the tool's rename of
/// it, and the budget. A batch prints the rows that fit, ends with one
/// over-budget failure whose fields are `left_out` and `budget`, and exits
/// with code 1. A batch's first row prints even when it passes the budget
/// alone if its document is marked with `Document::always_print_first`, and
/// is refused with its own over-budget failure otherwise (ADR-0084). A
/// single row, a prose result or a bytes result that does not fit prints
/// nothing but the over-budget failure, and exits 1, unless its document is
/// marked with `Document::always_print_first` (ADR-0046). A bytes result is
/// measured in the form it prints, so with `--json` its base64 text counts.
///
/// A single row, a `Vec` or a `Listing` whose row type has an empty
/// [`Row::COLUMNS`](crate::Row::COLUMNS) fails to compile, because its table
/// and its JSON without `--full` would print nothing of each row while
/// `total` counted the rows. A batch whose row type has no columns compiles,
/// because a headed batch needs none (ADR-0063).
///
/// When `--fields` names a field that the row type does not have, nothing of
/// the result prints. [`parse`](crate::parse()) refuses such a name before
/// the handler runs on a leaf that declares a document. The refusal prints as a failure does, with the message
/// `--fields names an unknown field 'radius'`, the code `<tool>::cli::usage`
/// in the guide's spelling, and the hint `use the valid fields: a, b`.
///
/// The exit code is 0 when the result is written, and also when the reader
/// closed the pipe early. When the result is a failure, or a batch or prose
/// result with at least one failure, the exit code belongs to the run's
/// first failure: the code of the guide's [`Exit`] whose
/// [`Exit::failures`] list that failure's printed code, or 1 when none
/// does, also after the reader closed the pipe early (ADR-0037). The
/// printed code is the tool's own code when the guide renames one of
/// crumbkit's with [`Guide::codes`](crate::help::Guide::codes). The guide reaches
/// `finish` through the `Output` that [`parse`](crate::parse()) or
/// [`Output::new`] returns. The exit code is
/// 1 as well when a write fails for any other reason, or when the
/// result cannot be serialized, and the cause of a failed write goes to the
/// sink's error writer before a batch's failures. It is the exit code of a
/// refused command line, 2 unless the guide lists another, when `--fields`
/// names an unknown field, or when the result cannot use a view flag that
/// the caller passed, which are checked before the result prints. When
/// `result` is a failure, `finish_to` checks no view flag and prints the
/// failure, because a tool may print an early failure in another document
/// than the one its leaf declares, and the `Output` already holds only a
/// view that every printer accepts (ADR-0093). A prose or bytes result
/// cannot use `--fields`, `--long` or `--full`, rows without long columns
/// cannot use `--long` unless the document lists only the rows that report
/// a problem or declares a long note (ADR-0139), rows of a type that sets
/// `Row::WHOLE` cannot use `--long` with `--json` (ADR-0138), and a
/// document without the envelope cannot use `--full` (ADR-0016).
///
/// A run without a failure whose rows report a problem, as
/// [`Row::is_problem`](crate::Row::is_problem) decides, exits with 1, also
/// after the reader closed the pipe early. The exit reads every row that
/// the result holds, including rows that the budget left unprinted, and a
/// listing from `Listing::from_all` also counts the rows that its limit
/// cut. In
/// text, a note such as `note: 2 of the aliases report a problem, so the
/// command exits with 1.` goes to the error writer after the footer and
/// before the note of a listing, a batch or a prose result (ADR-0050).
///
/// [books-show-7]: https://github.com/crumbkit/crumbkit/blob/main/examples/shelf/tests/golden/books-show-7.stderr
pub fn finish_to<T: Rows, E: AsFailure, O: Write, W: Write>(
    sink: &mut Sink<O, W>,
    mut output: Output,
    document: Document<T>,
    result: Result<T, E>,
) -> u8 {
    columns::require::<T>();
    // crumbkit's own codes and documents print with the tool's name in
    // place of `crumbkit` (ADR-0079).
    let (spelling, document) = output.named(document);
    // The sink counts the bytes of the result, which `finish` reports to the
    // observer with the exit code (ADR-0057).
    let truncated = result.as_ref().is_ok_and(listing::leaves_out);
    let empty = result.as_ref().is_ok_and(listing::matches_none);
    let before = sink.result_bytes();
    // With the `tracing` feature, the response runs inside the
    // `crumbkit.finish` span, which records the exit code (ADR-0030).
    #[cfg(feature = "tracing")]
    let span = crate::trace::spans::finish();
    #[cfg(feature = "tracing")]
    let _entered = span.enter();
    let (exit, cut) = respond::respond(sink, &mut output, spelling, document, result);
    #[cfg(feature = "tracing")]
    crate::trace::spans::exited(&span, exit);
    let bytes = sink.result_bytes() - before;
    crate::events::finished(exit, bytes, truncated || cut, empty);
    exit
}
