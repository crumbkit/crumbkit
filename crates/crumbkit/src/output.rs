//! The output flags, and `finish`, which prints a result in the format they
//! choose.

mod table;

use std::io::{self, Write};
use std::process::ExitCode;

use clap::Args;
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
#[derive(Args, Clone, Debug, Default, PartialEq, Eq)]
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
        write_document(out, document, value)?;
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
            [single] if T::SINGLE => write_document(out, document, single)?,
            list => write_document(out, document, &list)?,
        }
    } else {
        table::write(out, &columns, &rows)?;
    }
    out.flush()
}

/// Writes the JSON document `{"schema": <id>, "data": <data>}` to `out` on
/// one line.
fn write_document<T, D: Serialize>(
    out: &mut impl Write,
    document: Document<T>,
    data: &D,
) -> io::Result<()> {
    serde_json::to_writer(&mut *out, &Envelope { document, data })?;
    out.write_all(b"\n")
}

/// A JSON document: the `schema` id followed by the result under `data`.
struct Envelope<'a, T, D> {
    document: Document<T>,
    data: &'a D,
}

impl<T, D: Serialize> Serialize for Envelope<'_, T, D> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(2))?;
        // `format_args!` serializes as a string without building one.
        map.serialize_entry("schema", &format_args!("{}", self.document))?;
        map.serialize_entry("data", self.data)?;
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
