//! The `self runs export` command, which prints the lines of a tool's usage
//! log after a position, oldest first, as the log stores them (ADR-0191).
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
//! Each line names its position, `RUN@AT`, which is the run's span id and
//! its end time to the millisecond. A caller that keeps the last position
//! it read passes it to `--after` and reads only the lines that came after
//! it. The text prints one line per run:
//!
//! ```text
//! 04bdd5e48d990113@2026-10-01T09:00:00.000Z  shelf  0
//! ac0768ed50b138d0@2026-10-01T09:00:01.000Z  books  0
//! ```
//!
//! With `--json`, each line holds the stored `<tool>/usage/v1` record
//! under `record`, with every field the log stores, including its keyed
//! session and command-line hashes. The tool
//! sends nothing anywhere, and the person who runs the export decides where
//! its output goes.

use std::borrow::Cow;
use std::fmt;
use std::io::{BufRead, Seek, SeekFrom};
use std::ops::ControlFlow;
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;
use std::time::{Duration, UNIX_EPOCH};

use clap::Args;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::Serialize;
use serde_json::Value;

use super::record::Record;
use super::report::{Entry, is_hex, lines, millis_of};
use super::self_usage::open_log;
use crumbkit::{Document, Run, Stream};

#[cfg(test)]
mod tests;

/// The description of [`SELF_RUNS_EXPORT`].
const DESCRIPTION: &str = "Each line of the usage log after a position, oldest first, with the record that the log stores.";

/// The document that `self runs export` prints: one line for each record
/// in the usage log after the position that `--after` names. Its schema is
/// `crates/crumbkit-usage/schemas/self-runs-export-v1.json`, the schema of
/// one line.
pub const SELF_RUNS_EXPORT: Document<Stream<RunLine>> =
    Document::new("crumbkit", "self-runs-export", 1, DESCRIPTION)
        .when_empty("the usage log holds no runs after the position");

/// The arguments of `<tool> self runs export`: the position to print the
/// lines after, and the log to read when the caller names one.
#[derive(Args, Clone, Debug, PartialEq, Eq)]
// The doc comment is rustdoc, and clap would copy it into the help of a
// command without a description of its own.
#[command(about = None, long_about = None)]
pub struct SelfRunsExport {
    /// Print only the lines after this position, which each line gives, such as 00f067aa0ba902b7@2026-10-11T11:56:40.120Z
    #[arg(long, value_name = "RUN@AT")]
    after: Option<Position>,

    /// Read this usage log instead of the tool's own
    #[arg(long, value_name = "FILE")]
    log: Option<PathBuf>,
}

/// A place in the usage log: the span id of a run and the time it ended, in
/// milliseconds since the Unix epoch. It reads and prints as `RUN@AT`,
/// such as `00f067aa0ba902b7@2026-10-11T11:56:40.120Z`.
///
/// The log has no stable byte offset or line number, because a trim
/// rewrites the file, so a run names the place. The span id is random, and
/// the end time makes a match with another run's id impossible in practice.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Position {
    /// The run's span id, as 16 lowercase hex digits.
    run: String,
    /// When the run ended, in milliseconds since the Unix epoch.
    at: u64,
}

impl Position {
    /// Returns the position of the run that `entry` records.
    fn of(entry: &Entry) -> Self {
        Self {
            run: entry.run.clone(),
            at: entry.at,
        }
    }

    /// Returns whether `entry` records the run at this position.
    fn names(&self, entry: &Entry) -> bool {
        entry.at == self.at && entry.run == self.run
    }
}

impl FromStr for Position {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let refused = || {
            "a position is RUN@AT: a run's span id as 16 lowercase hex digits, `@`, and the run's end time in RFC 3339, such as 00f067aa0ba902b7@2026-10-11T11:56:40.120Z".to_owned()
        };
        let (run, at) = text.split_once('@').ok_or_else(refused)?;
        if !is_hex(run, 16) {
            return Err(refused());
        }
        let at = millis_of(at).ok_or_else(refused)?;
        Ok(Self {
            run: run.to_owned(),
            at,
        })
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The reader parsed the time from RFC 3339, so humantime formats it
        // without a panic.
        let end = UNIX_EPOCH + Duration::from_millis(self.at);
        write!(f, "{}@{}", self.run, humantime::format_rfc3339_millis(end))
    }
}

/// One line of the usage log as `self runs export` prints it.
#[derive(Debug, Serialize, JsonSchema)]
pub struct RunLine {
    /// The line's position, RUN@AT: the run's span id and its end time to
    /// the millisecond, such as `00f067aa0ba902b7@2026-10-11T11:56:40.120Z`.
    /// `--after` takes it to print the lines that follow this one.
    position: String,
    /// Whether lines may be missing before this one, because the run that
    /// `--after` named is no longer in the log, and the export started at
    /// that run's end time instead. Only the first line of an export can be
    /// true.
    gap: bool,
    /// The line's record with every field and value that the usage log
    /// stores, including the keyed hashes of the run's session and command
    /// line.
    record: StoredRecord,
    /// The command path below the tool's name, or the tool's name for its
    /// root command, which the text line prints.
    #[serde(skip)]
    command: String,
    /// The run's exit code, which the text line prints.
    #[serde(skip)]
    exit: Option<u8>,
}

impl crumbkit::Row for RunLine {
    const COLUMNS: &'static [&'static str] = &["position", "gap", "record"];
    const LINE: Option<&'static str> = Some("<POSITION>  <COMMAND>  <EXIT>  [after a gap]");

    fn text_line(&self) -> Option<String> {
        let exit = self
            .exit
            .map_or_else(|| "-".to_owned(), |exit| exit.to_string());
        let gap = if self.gap { "  after a gap" } else { "" };
        Some(format!("{}  {}  {exit}{gap}", self.position, self.command))
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

/// Prints each line of the usage log of the tool whose command `run`
/// holds, after the position that `args` names, as the stream
/// [`SELF_RUNS_EXPORT`], and returns the process's exit code.
///
/// It reads the file that `--log` names, or else `usage.jsonl` in the
/// tool's directory under the directory where [`start`](super::start)
/// writes it, oldest line first. It prints each line that is a whole
/// `<tool>/usage/v1` record and ends with a newline. A last line without a
/// newline is a run's write in progress, which the next export prints, so
/// no half line prints. It takes no lock, so a run that appends while the
/// export reads never waits for it.
///
/// Without `--after`, it prints every record. With `--after`, it prints the
/// records that follow the run at that position. When no record in the log
/// is at that position, because a trim removed it, it prints every record
/// that ended at or after the position's time, and marks the first line
/// with `gap`, because records between the position and the oldest kept
/// record may be missing. Such an export may repeat a run that ended in the
/// same millisecond as the position, which a reader drops by its span id.
///
/// The stream prints within the tool's output budget. When the next line
/// would pass it, the stream prints the over-budget failure and exits with
/// 1, and the caller exports again after the last position it read. A
/// failure to find or read the log stays silent, as every usage-log
/// failure does, and the stream holds the lines read before it.
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
        each_line(&name, log, args.after.as_ref(), |line| {
            streaming.item(&line)
        });
    }
    streaming.end(Ok::<(), crumbkit::Error>(()))
}

/// Passes each line of `log` after `after` to `item`, as [`export`]
/// describes, for the tool named `tool`, until `item` returns
/// `ControlFlow::Break` or the log ends.
///
/// It reads the log once when the position is in it, and goes on from that
/// line. When the position is not in it, it reads the log a second time
/// from the start through the same handle, which a trim's rename leaves on
/// the file that it read the first time.
fn each_line<L: BufRead + Seek>(
    tool: &str,
    mut log: L,
    after: Option<&Position>,
    mut item: impl FnMut(RunLine) -> ControlFlow<()>,
) {
    let Some(after) = after else {
        lines(log, |line| print(tool, line, &mut item));
        return;
    };
    let mut found = false;
    lines(&mut log, |line| {
        if found {
            return print(tool, line, &mut item);
        }
        found = whole(line).is_some_and(|(entry, _)| after.names(&entry));
        ControlFlow::Continue(())
    });
    if found || log.seek(SeekFrom::Start(0)).is_err() {
        return;
    }
    let mut gap = true;
    lines(log, |line| {
        let Some((entry, text)) = whole(line).filter(|(entry, _)| entry.at >= after.at) else {
            return ControlFlow::Continue(());
        };
        let Some(row) = RunLine::of(tool, entry, text, gap) else {
            return ControlFlow::Continue(());
        };
        gap = false;
        item(row)
    });
}

/// Passes the line that prints `line` of the log of the tool named `tool`
/// to `item` when `line` is a whole record, and returns what `item`
/// returns, or `ControlFlow::Continue` for any other line.
fn print(
    tool: &str,
    line: &[u8],
    item: &mut impl FnMut(RunLine) -> ControlFlow<()>,
) -> ControlFlow<()> {
    whole(line)
        .and_then(|(entry, text)| RunLine::of(tool, entry, text, false))
        .map_or(ControlFlow::Continue(()), item)
}

/// Returns the record that `line` holds and its text without the newline,
/// or `None` when the line has no newline, because a run is still writing
/// it, or is not a whole usage record.
fn whole(line: &[u8]) -> Option<(Entry, &str)> {
    let text = std::str::from_utf8(line.strip_suffix(b"\n")?).ok()?;
    Some((Entry::parse(text.as_bytes())?, text))
}

impl RunLine {
    /// Returns the line that prints `entry`, whose text in the log is
    /// `text`, for the tool named `tool`, marked as after a gap when `gap`
    /// is true, or `None` when `text` is not JSON.
    fn of(tool: &str, entry: Entry, text: &str, gap: bool) -> Option<Self> {
        let record = serde_json::from_str(text).ok()?;
        Some(Self {
            position: Position::of(&entry).to_string(),
            gap,
            record: StoredRecord(record),
            command: entry.command.unwrap_or_else(|| tool.to_owned()),
            exit: entry.exit,
        })
    }
}
