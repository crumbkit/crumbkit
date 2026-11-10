//! The `self runs list` command, which prints each run in a tool's usage
//! log, grouped by session and in order within each session (ADR-0029,
//! ADR-0190).
//!
//! The tool owns the command. It adds a `runs` resource with a `list` verb
//! that takes [`SelfRuns`] to its `self` group, and its handler passes the
//! result of [`runs`] to [`finish`](crumbkit::finish) with the document
//! [`SELF_RUNS_LIST`]:
//!
//! ```ignore
//! SelfCommand::Runs { verb: RunsVerb::List(args) } => {
//!     let result = crumbkit_usage::runs(&mut run, &TOOL, &args);
//!     let result = Ok::<_, crumbkit::Error>(result);
//!     crumbkit::finish(run, crumbkit_usage::SELF_RUNS_LIST, result)
//! }
//! ```
//!
//! The listing prints one row for each run, with the summary line of
//! `self usage` above the table:
//!
//! ```text
//! 3 runs in 2 sessions since 4 Oct: person 3.
//! SESSION  STEP  COMMAND     EXIT  CODE                     MS  OUT_BYTES
//! 1        1     shelf       0     -                        10  -
//! 1        2     books list  0     -                        12  512
//! 2        1     books show  1     shelf::books::not_found  9   86
//! ```
//!
//! It numbers the sessions from 1 and never prints a session's keyed hash.

use std::collections::HashMap;
use std::io::BufRead;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use clap::{Args, Command};
use schemars::JsonSchema;
use serde::Serialize;

use super::report::Entry;
use super::self_usage::{Selection, SelfUsage, UsageSummary, open, summarize};
use crumbkit::{Document, Listing, Page, Run, Tool};

#[cfg(test)]
mod tests;

/// The document that `self runs list` prints: one row for each run in the
/// usage log, and the summary of the runs that it read. Its schema is
/// `crates/crumbkit-usage/schemas/self-runs-list-v1.json`, which
/// [`schemas::document_schema`](crumbkit::schemas::document_schema) builds.
pub const SELF_RUNS_LIST: Document<Listing<RunRow, UsageSummary>> = Document::new(
    "crumbkit",
    "self-runs-list",
    1,
    "Each run in the usage log, grouped by session and in order within each session, and a summary of the runs read.",
)
.built_in();

/// The arguments of `<tool> self runs list`: the arguments of
/// [`SelfUsage`], and how many rows to print.
#[derive(Args, Clone, Debug, PartialEq, Eq)]
// The doc comment is rustdoc, and clap would copy it into the help of a
// command without a description of its own.
#[command(about = None, long_about = None)]
pub struct SelfRuns {
    /// How far back to read and which log to read.
    #[command(flatten)]
    usage: SelfUsage,

    /// How many runs to print.
    #[command(flatten)]
    page: Page,
}

/// One run in the usage log.
#[derive(Debug, Serialize, JsonSchema)]
pub struct RunRow {
    /// The run's session, numbered from 1 in the order of each session's
    /// first run in the listing's window.
    session: u64,
    /// The run's place in its session, from 1.
    step: u64,
    /// When the run ended, in RFC 3339 to the second in UTC, such as
    /// `2026-10-11T11:56:40Z`.
    at: String,
    /// The coding agent's id, such as claude-code, person or unknown.
    caller: String,
    /// The command path below the tool's name, such as `books show`, or the
    /// tool's name for its root command.
    command: String,
    /// The names of the flags that the caller typed, never their values.
    flags: Vec<String>,
    /// Whether the run printed a help page.
    help: bool,
    /// The run's exit code, or null when the run ended before it exited.
    exit: Option<u8>,
    /// The code of the run's failure or refusal, such as
    /// `shelf::books::not_found`, or null when the run recorded none.
    code: Option<String>,
    /// The run's wall time in milliseconds.
    ms: u64,
    /// The bytes that the run wrote to standard output, or null when the
    /// run did not reach `finish`.
    out_bytes: Option<u64>,
    /// Whether a listing or stream cut the run's output short.
    truncated: bool,
    /// The id of the W3C trace that the run belongs to, as 32 lowercase hex
    /// digits.
    trace: String,
    /// The run's own span id, as 16 lowercase hex digits, which its child
    /// runs name as their parent.
    run: String,
    /// The span id of the run's caller, as 16 lowercase hex digits, or null
    /// when the run started a new trace.
    parent: Option<String>,
}

impl crumbkit::Row for RunRow {
    const COLUMNS: &'static [&'static str] = &[
        "session",
        "step",
        "command",
        "exit",
        "code",
        "ms",
        "out_bytes",
    ];
    const LONG_COLUMNS: &'static [&'static str] = &[
        "at",
        "caller",
        "flags",
        "help",
        "truncated",
        "trace",
        "run",
        "parent",
    ];
}

/// Returns each run in the usage log of the tool whose command `run` holds,
/// for the runs that ended in the `--since` duration before now, one page
/// of them as `args` asks, for the tool to pass to
/// [`finish`](crumbkit::finish) with [`SELF_RUNS_LIST`].
///
/// It reads the log that [`findings`](super::findings) reads, in the same
/// way, leaves out and counts the same records, and leaves out the runs of
/// each program that `--exclude-program` names. The rows are grouped
/// by session, the session whose first run came first in the log first,
/// and follow the log's order within each session. `tool` is the tool
/// record, which gives the exit code of a refused command line for the
/// summary.
///
/// It holds a row for every run in the window, so its memory grows with
/// the runs that `--since` admits. A 10 MB log holds about 29,000 runs.
#[must_use]
pub fn runs(run: &mut Run, tool: &Tool, args: &SelfRuns) -> Listing<RunRow, UsageSummary> {
    let tool_name = run.command();
    runs_from(
        tool_name,
        tool,
        open(tool_name, &args.usage),
        &args.usage.selection(SystemTime::now()),
        args.page.limit(),
    )
}

/// Returns the runs in `log` that fall in `selection`, checked against `tool`,
/// with at most `limit` rows, as [`runs`] describes for `tool`.
fn runs_from(
    tool_name: &Command,
    tool: &Tool,
    log: Option<impl BufRead>,
    selection: &Selection,
    limit: Option<usize>,
) -> Listing<RunRow, UsageSummary> {
    let name = tool_name.get_name();
    // The ordinal of each session by its keyed hash, and each session's
    // rows. The hash stays in the map and never reaches a row.
    let mut ordinals: HashMap<String, usize> = HashMap::new();
    let mut sessions: Vec<Vec<RunRow>> = Vec::new();
    let (_, summary) = summarize(tool_name, tool, log, selection, |entry| {
        let index = *ordinals.entry(entry.session.clone()).or_insert_with(|| {
            sessions.push(Vec::new());
            sessions.len() - 1
        });
        if let Some(rows) = sessions.get_mut(index) {
            let row = row(name, index, rows.len(), entry);
            rows.push(row);
        }
    });
    let rows = sessions.into_iter().flatten().collect();
    Listing::from_all(rows, limit)
        .described("runs")
        .summary(summary)
}

/// Returns the row of `entry`, the run at `place` from 0 in the session at
/// `index` from 0, for the tool named `tool_name`.
fn row(tool_name: &str, index: usize, place: usize, entry: Entry) -> RunRow {
    let end = UNIX_EPOCH + Duration::from_millis(entry.at);
    RunRow {
        session: index as u64 + 1,
        step: place as u64 + 1,
        // The reader parsed the time from RFC 3339, so humantime formats it
        // without a panic.
        at: humantime::format_rfc3339_seconds(end).to_string(),
        caller: entry.caller,
        command: entry.command.unwrap_or_else(|| tool_name.to_owned()),
        flags: entry.flags,
        help: entry.help,
        exit: entry.exit,
        code: entry.code,
        ms: entry.ms,
        out_bytes: entry.out_bytes,
        truncated: entry.truncated,
        trace: entry.trace,
        run: entry.run,
        parent: entry.parent,
    }
}
