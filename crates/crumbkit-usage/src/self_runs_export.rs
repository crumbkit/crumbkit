//! The `self runs export` command, which prints the records of a tool's
//! usage log after a run, oldest first, as the log stores them (ADR-0191).
//!
//! The tool owns the command. It adds an `export` verb that takes
//! [`SelfRunsExport`] to the `runs` resource of its `self` group, and its
//! handler returns the exit code of [`export`], which prints the stream
//! [`SELF_RUNS_EXPORT`]:
//!
//! ```ignore
//! SelfCommand::Runs { command: RunsCommand::Export(args) } => {
//!     crumbkit_usage::export(run, &args)
//! }
//! ```
//!
//! A caller that keeps the span id of the last run it read, the `run` of
//! the last record, passes it to `--after` and reads only the records that
//! came after it. The text prints one line per run:
//!
//! ```text
//! 04bdd5e48d990113  shelf  0
//! ac0768ed50b138d0  books  0
//! ```
//!
//! With `--json`, each line holds the stored `<tool>/usage/v1` record
//! under `record`, with every field the log stores, including its keyed
//! session and command-line hashes. The tool sends nothing anywhere, and
//! the person who runs the export decides where its output goes.

use std::borrow::Cow;
use std::io::{BufRead, Seek, SeekFrom};
use std::ops::ControlFlow;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Args;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::Serialize;
use serde_json::Value;

use super::record::Record;
use super::report::{Entry, is_hex, lines};
use super::self_usage::open_log;
use crumbkit::{Document, Run, Stream};

#[cfg(test)]
mod tests;

/// The description of [`SELF_RUNS_EXPORT`].
const DESCRIPTION: &str =
    "Each record of the usage log after a run, oldest first, as the log stores it.";

/// The document that `self runs export` prints: one line for each record
/// in the usage log after the run that `--after` names. Its schema is
/// `crates/crumbkit-usage/schemas/self-runs-export-v1.json`, the schema of
/// one line.
pub const SELF_RUNS_EXPORT: Document<Stream<RunLine>> =
    Document::new("crumbkit", "self-runs-export", 1, DESCRIPTION)
        .when_empty("the usage log holds no runs after the one named")
        .built_in();

/// The arguments of `<tool> self runs export`: the run to print the records
/// after, and the log to read when the caller names one.
#[derive(Args, Clone, Debug, PartialEq, Eq)]
// The doc comment is rustdoc, and clap would copy it into the help of a
// command without a description of its own.
#[command(about = None, long_about = None)]
pub struct SelfRunsExport {
    /// Print only the records after the run with this span id, the `run` of the last record an earlier export printed
    #[arg(long, value_name = "RUN", value_parser = span_id)]
    after: Option<String>,

    /// Read this usage log instead of the tool's own
    #[arg(long, value_name = "FILE")]
    log: Option<PathBuf>,
}

/// Returns `text` when it is a span id, 16 lowercase hex digits.
///
/// # Errors
///
/// Returns why `text` is refused when it is not a span id.
fn span_id(text: &str) -> Result<String, String> {
    if is_hex(text, 16) {
        Ok(text.to_owned())
    } else {
        Err("a run is its span id, 16 lowercase hex digits, such as 00f067aa0ba902b7".to_owned())
    }
}

/// One record of the usage log as `self runs export` prints it.
#[derive(Debug, Serialize, JsonSchema)]
pub struct RunLine {
    /// Whether records may be missing before this one, because the run that
    /// `--after` named is no longer in the log, so the export printed every
    /// record the log holds. Only the first line of an export can be true.
    gap: bool,
    /// The record with every field and value that the usage log stores,
    /// including the run's span id under `run`, which `--after` takes, and
    /// the keyed hashes of the run's session and command line.
    record: StoredRecord,
    /// The run's span id, which the text line prints.
    #[serde(skip)]
    run: String,
    /// The command path below the tool's name, or the tool's name for its
    /// root command, which the text line prints.
    #[serde(skip)]
    command: String,
    /// The run's exit code, which the text line prints.
    #[serde(skip)]
    exit: Option<u8>,
}

impl crumbkit::Row for RunLine {
    const COLUMNS: &'static [&'static str] = &["gap", "record"];
    const LINE: Option<&'static str> = Some("<RUN>  <COMMAND>  <EXIT>  [after a gap]");

    fn text_line(&self) -> Option<String> {
        let exit = self
            .exit
            .map_or_else(|| "-".to_owned(), |exit| exit.to_string());
        let gap = if self.gap { "  after a gap" } else { "" };
        Some(format!("{}  {}  {exit}{gap}", self.run, self.command))
    }
}

/// One `<tool>/usage/v1` record, with every field and value that the usage
/// log stores for it. The JSON output orders the keys by name, as crumbkit's
/// JSON does, so the line's text may differ from the stored line's.
#[derive(Debug, Serialize)]
#[serde(transparent)]
struct StoredRecord(Value);

impl JsonSchema for StoredRecord {
    fn inline_schema() -> bool {
        true
    }

    fn schema_name() -> Cow<'static, str> {
        "StoredRecord".into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        generator.subschema_for::<Record>()
    }
}

/// Prints each record of the usage log of the tool whose command `run`
/// holds, after the run that `args` names, as the stream
/// [`SELF_RUNS_EXPORT`], and returns the process's exit code.
///
/// It reads the file that `--log` names, or else `usage.jsonl` in the
/// tool's directory under the directory where [`start`](super::start)
/// writes it, oldest line first. It prints each line that is a whole
/// `<tool>/usage/v1` record with a span id of 16 lowercase hex digits and
/// ends with a newline. A last line without a newline is a run's write in
/// progress, which the next export prints, so no half line prints. It
/// takes no lock, so a run that appends while the export reads never waits
/// for it.
///
/// Without `--after`, it prints every record. With `--after`, it prints the
/// records that follow the run with that span id. When no record in the
/// log has it, because a trim removed it, it prints every record, and marks
/// the first line with `gap`. A trim keeps the newest lines, which all came
/// after the removed run, so the export repeats none, and the records
/// between that run and the oldest kept one are missing.
///
/// The stream prints within the tool's output budget. When the next line
/// would pass it, the stream prints the over-budget failure and exits with
/// 1, and the caller exports again after the last run it read. A failure
/// to find or read the log stays silent, as every usage-log failure does,
/// and the stream holds the lines read before it.
#[must_use]
pub fn export(mut run: Run, args: &SelfRunsExport) -> ExitCode {
    let tool = run.command();
    let name = tool.get_name().to_owned();
    let log = open_log(tool, args.log.as_deref());
    let mut streaming = match crumbkit::stream(run, SELF_RUNS_EXPORT) {
        Ok(streaming) => streaming,
        Err(code) => return code,
    };
    if let Some(log) = log {
        each_line(&name, log, args.after.as_deref(), |line| {
            streaming.item(&line)
        });
    }
    streaming.end(Ok::<(), crumbkit::Error>(()))
}

/// Passes each record of `log` after the run `after` to `item`, as
/// [`export`] describes, for the tool named `tool`, until `item` returns
/// `ControlFlow::Break` or the log ends.
///
/// It reads the log once when the run is in it, and goes on from that
/// line. When the run is not in it, it reads the log a second time from the
/// start through the same handle, which a trim's rename leaves on the file
/// that it read the first time.
fn each_line<L: BufRead + Seek>(
    tool: &str,
    mut log: L,
    after: Option<&str>,
    mut item: impl FnMut(RunLine) -> ControlFlow<()>,
) {
    let mut gap = false;
    if let Some(after) = after {
        let mut found = false;
        lines(&mut log, |line| {
            if found {
                return print(tool, line, &mut gap, &mut item);
            }
            found = whole(line).is_some_and(|(entry, _)| entry.run == after);
            ControlFlow::Continue(())
        });
        if found || log.seek(SeekFrom::Start(0)).is_err() {
            return;
        }
        gap = true;
    }
    lines(log, |line| print(tool, line, &mut gap, &mut item));
}

/// Passes the line that prints `line` of the log of the tool named `tool`
/// to `item` when `line` is a whole record, marked as after a gap when
/// `gap` is true, which it then clears. It returns what `item` returns, or
/// `ControlFlow::Continue` for any other line.
fn print(
    tool: &str,
    line: &[u8],
    gap: &mut bool,
    item: &mut impl FnMut(RunLine) -> ControlFlow<()>,
) -> ControlFlow<()> {
    let Some(row) = whole(line).and_then(|(entry, text)| RunLine::of(tool, entry, text, *gap))
    else {
        return ControlFlow::Continue(());
    };
    *gap = false;
    item(row)
}

/// Returns the record that `line` holds and its text without the newline,
/// or `None` when the line has no newline, because a run is still writing
/// it, is not a whole usage record, or names a span id that is not 16
/// lowercase hex digits, which `--after` would refuse.
fn whole(line: &[u8]) -> Option<(Entry, &str)> {
    let text = std::str::from_utf8(line.strip_suffix(b"\n")?).ok()?;
    let entry = Entry::parse(text.as_bytes()).filter(|entry| is_hex(&entry.run, 16))?;
    Some((entry, text))
}

impl RunLine {
    /// Returns the line that prints `entry`, whose text in the log is
    /// `text`, for the tool named `tool`, marked as after a gap when `gap`
    /// is true, or `None` when `text` is not JSON.
    fn of(tool: &str, entry: Entry, text: &str, gap: bool) -> Option<Self> {
        let record = serde_json::from_str(text).ok()?;
        Some(Self {
            gap,
            record: StoredRecord(record),
            run: entry.run,
            command: entry.command.unwrap_or_else(|| tool.to_owned()),
            exit: entry.exit,
        })
    }
}
