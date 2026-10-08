//! The output flags, and `finish`, which prints a result in the format they
//! choose.

mod table;

use std::io::{self, Write};
use std::process::ExitCode;

use clap::{Args, Command};
use serde::Serialize;
use serde::ser::{SerializeMap, Serializer};
use serde_json::Value;

use crate::{Document, Row, Rows, Sink};

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
/// - `--full` prints every field of every row, and requires `--json`.
/// - `--fields a,b` prints exactly the named fields in that order, and
///   conflicts with `--long` and `--full`.
///
/// It also holds the command tree that the tool parsed against, which
/// [`Output::set_command`] sets, so `finish` can render a listing's next step.
///
/// `Output` derives `Clone`, `Debug` and `Default`. It does not derive
/// `PartialEq` or `Eq`, because `clap::Command` implements neither and nothing
/// compares two `Output` values.
#[derive(Args, Clone, Debug, Default)]
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

    /// The command tree that the tool parsed against, built, or `None` until
    /// the tool calls `set_command`. clap skips the field when it parses.
    #[arg(skip)]
    command: Option<Command>,
}

impl Output {
    /// Keeps `command` so that [`finish`] can render a listing's next step
    /// against it. A tool's `main` calls it once after parsing, with the same
    /// command tree it parsed against.
    ///
    /// It calls `Command::build`, which clap requires before a command's
    /// arguments are read, and which does nothing to a command that parsing
    /// already built. Without a command, `finish` leaves out the `Next:`
    /// line.
    ///
    /// A tool that does not change its tree passes `Cli::command()`:
    ///
    /// ```
    /// use clap::{CommandFactory, Parser};
    /// use crumbkit::Output;
    ///
    /// /// List the books in a small catalogue.
    /// #[derive(Parser)]
    /// struct Cli {
    ///     #[command(flatten)]
    ///     output: Output,
    /// }
    ///
    /// let mut cli = Cli::parse_from(["shelf"]);
    /// cli.output.set_command(Cli::command());
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
    pub fn set_command(&mut self, mut command: Command) {
        command.build();
        self.command = Some(command);
    }
}

/// Prints `value` to standard output in the format that `output` chooses,
/// and returns the process's exit code.
///
/// A tool's `main` calls it last and returns its exit code. It behaves as
/// [`finish_to`] does with a sink over the process's standard output and
/// standard error.
pub fn finish<T: Rows>(output: &Output, document: Document<T>, value: T) -> ExitCode {
    finish_to(&mut Sink::standard(), output, document, value)
}

/// Prints `value` to `sink` in the format that `output` chooses, and returns
/// the exit code that [`finish`] would return.
///
/// Tests and golden files call it with a sink over in-memory writers, so they
/// check a command's exact output and exit code without starting a process.
///
/// The text format is a table with a header row. The JSON format is one line
/// holding an object with the `schema` id of `document` and the result under
/// `data`; a single row's `data` is an object, and a `Vec`'s is an array.
///
/// A [`Listing`](crate::Listing)'s `data` is an array, and its JSON document
/// also holds the number of matching rows under `total`. When a listing
/// shows fewer rows than its total, the table ends with a footer line, such
/// as `Showing 4 of 6 books. --all shows every one.` When a listing has a
/// next step, a `Next:` line follows the table and any footer, such as
/// `Next: shelf books show <ID>... takes up to 100 ids in one call.` The line
/// is left out when [`Output::set_command`] was not called or the step's
/// [`Call`](crate::Call) does not resolve in that command, and the exit code
/// stays 0.
///
/// The exit code is 0 when the result is written, and also when the reader
/// closed the pipe early. It is 1 when a write fails for any other reason, or
/// when the result cannot be serialized, and the cause goes to the sink's
/// error writer. It is 2 when `--fields` names a field that the row type does
/// not have.
#[expect(
    clippy::needless_pass_by_value,
    reason = "finish_to takes the result by value, as finish does, so a tool passes the result it built in the same way to either"
)]
pub fn finish_to<T: Rows, O: Write, E: Write>(
    sink: &mut Sink<O, E>,
    output: &Output,
    document: Document<T>,
    value: T,
) -> ExitCode {
    if let Some(message) = unknown_field::<T::Row>(&output.fields) {
        // The process is already failing, and a failed report has nowhere
        // else to go.
        let _ = writeln!(sink.err, "error: {message}");
        return ExitCode::from(2);
    }
    match write_result(&mut sink.out, output, document, &value) {
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
    }
}

/// Returns the error message for the first name in `fields` that is not a
/// field of `R`, or `None` when every name is a field.
///
/// The fields of `R` are the properties of its JSON Schema, so a mistyped
/// name fails even when the result has no rows.
fn unknown_field<R: Row>(fields: &[String]) -> Option<String> {
    if fields.is_empty() {
        return None;
    }
    let schema = schemars::schema_for!(R);
    let properties = schema.get("properties").and_then(Value::as_object);
    let is_field = |name: &str| properties.is_some_and(|properties| properties.contains_key(name));
    let unknown = fields.iter().find(|name| !is_field(name))?;

    // serde_json keeps an object's keys sorted unless a crate in the build
    // enables its `preserve_order` feature, which keeps the schema's order.
    let valid: Vec<&str> = properties
        .into_iter()
        .flat_map(|properties| properties.keys())
        .map(String::as_str)
        .collect();
    Some(format!(
        "--fields names an unknown field '{unknown}'; the valid fields are {}",
        valid.join(", ")
    ))
}

/// Writes `value` to `out` in the format that `output` chooses, then flushes
/// `out`.
///
/// A serialization error becomes an `io::Error` of kind `InvalidData`, so it
/// is reported as a failed write.
fn write_result<T: Rows>(
    out: &mut impl Write,
    output: &Output,
    document: Document<T>,
    value: &T,
) -> io::Result<()> {
    if output.full {
        // Serializing the rows directly keeps every field in its declaration
        // order.
        write_document(out, document, value, value.total())?;
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
            [single] if T::SINGLE => write_document(out, document, single, value.total())?,
            list => write_document(out, document, &list, value.total())?,
        }
    } else {
        table::write(out, &columns, &rows)?;
        write_footer(out, value)?;
        write_next(out, output.command.as_ref(), value)?;
    }
    out.flush()
}

/// Writes the footer line that follows the table when `value` is a listing
/// that shows fewer rows than its total, such as `Showing 4 of 6 books. --all
/// shows every one.` Otherwise it writes nothing.
fn write_footer<T: Rows>(out: &mut impl Write, value: &T) -> io::Result<()> {
    let shown = value.rows().len();
    let Some(total) = value.total() else {
        return Ok(());
    };
    if shown >= total {
        return Ok(());
    }
    match value.description() {
        Some(description) => writeln!(
            out,
            "Showing {shown} of {total} {description}. --all shows every one."
        ),
        None => writeln!(out, "Showing {shown} of {total}. --all shows every one."),
    }
}

/// Writes the line `Next: {call} {phrase}` when `value` is a listing with a
/// next step whose call resolves in `command`. Otherwise it writes nothing:
/// without a command, or when the call names a missing or hidden command or
/// argument, the tool still prints its result and crumbkit advertises
/// nothing the tool hid.
fn write_next<T: Rows>(
    out: &mut impl Write,
    command: Option<&Command>,
    value: &T,
) -> io::Result<()> {
    let (Some(command), Some((call, phrase))) = (command, value.next_step()) else {
        return Ok(());
    };
    match call.render_built(command) {
        Ok(rendered) => writeln!(out, "Next: {rendered} {phrase}"),
        Err(_) => Ok(()),
    }
}

/// Writes the JSON document `{"schema": <id>, "data": <data>}` to `out` on
/// one line, with `"total": <total>` after `data` when `total` is `Some`.
fn write_document<T, D: Serialize>(
    out: &mut impl Write,
    document: Document<T>,
    data: &D,
    total: Option<usize>,
) -> io::Result<()> {
    let envelope = Envelope {
        document,
        data,
        total,
    };
    serde_json::to_writer(&mut *out, &envelope)?;
    out.write_all(b"\n")
}

/// A JSON document: the `schema` id, the result under `data`, and a
/// listing's number of matching rows under `total`.
struct Envelope<'a, T, D> {
    document: Document<T>,
    data: &'a D,
    total: Option<usize>,
}

impl<T, D: Serialize> Serialize for Envelope<'_, T, D> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let len = if self.total.is_some() { 3 } else { 2 };
        let mut map = serializer.serialize_map(Some(len))?;
        // `format_args!` serializes as a string without building one.
        map.serialize_entry("schema", &format_args!("{}", self.document))?;
        map.serialize_entry("data", self.data)?;
        if let Some(total) = self.total {
            map.serialize_entry("total", &total)?;
        }
        map.end()
    }
}

/// One serialized row, limited to `columns` and written in their order.
struct Selected<'a> {
    row: &'a Value,
    columns: &'a [&'a str],
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
