//! The `self usage` command, which reports the patterns in a tool's usage
//! log where callers waste effort (ADR-0029).
//!
//! The tool owns the command. It adds a `self` group with a `usage`
//! subcommand that takes [`SelfUsage`] to its own clap types, and its
//! handler passes the result of [`findings`] to [`finish`](crumbkit::finish)
//! with the document [`SELF_USAGE`]:
//!
//! ```ignore
//! Resource::SelfCommand { command: SelfCommand::Usage(args) } => {
//!     let result = crumbkit_usage::findings(&mut output, &GUIDE, &args);
//!     let result = Ok::<_, crumbkit::Error>(result);
//!     crumbkit::finish(output, crumbkit_usage::SELF_USAGE, result)
//! }
//! ```
//!
//! The report prints one row for each finding, with a summary line above
//! the table:
//!
//! ```text
//! 1,284 runs in 23 sessions since 30 Sep: claude-code 1,201, person 83.
//! FINDING           RUNS  COMMAND     SUGGESTION
//! one id at a time  212   books show  pass every id to one call
//! identical repeat  88    books list  the same call ran again within 60 s
//! Next: shelf self usage --json --full adds the figures of each finding's command.
//! ```
//!
//! It never prints a hash, a session, or a time finer than a day.

use std::borrow::Cow;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use clap::{Args, Command};
use schemars::JsonSchema;
use serde::Serialize;

use super::file::LOG;
use super::report::{Figures, Finding, Kind, Report, Tally, read};
use crumbkit::help::Guide;
use crumbkit::{Call, Document, Listing, Output};

#[cfg(test)]
mod tests;

/// The document that `self usage` prints: one row for each finding, and
/// the summary of the runs that the report read. Its schema is
/// `crates/crumbkit-usage/schemas/self-usage-v1.json`, which
/// [`schemas::document_schema`](crumbkit::schemas::document_schema) builds.
pub const SELF_USAGE: Document<Listing<FindingRow, UsageSummary>> = Document::new(
    "crumbkit",
    "self-usage",
    1,
    "The patterns in the usage log where callers waste effort, and a summary of the runs read.",
)
.when_empty("the usage log holds no findings");

/// The arguments of `<tool> self usage`: how far back the report reads,
/// and the log it reads when the caller names one.
#[derive(Args, Clone, Debug, PartialEq, Eq)]
// The doc comment is rustdoc, and clap would copy it into the help of a
// command without a description of its own.
#[command(about = None, long_about = None)]
pub struct SelfUsage {
    /// How far back to read the usage log, such as 12h, 7d or 2w
    #[arg(long, value_name = "DURATION", default_value = "7d", value_parser = humantime::parse_duration)]
    pub(super) since: Duration,

    /// Read this usage log instead of the tool's own
    #[arg(long, value_name = "FILE")]
    log: Option<PathBuf>,
}

/// One finding of `self usage`: a pattern in the runs of one command that
/// suggests where callers waste effort.
#[derive(Debug, Serialize, JsonSchema)]
pub struct FindingRow {
    /// What the pattern is.
    finding: Kind,
    /// The runs that show the pattern.
    runs: u64,
    /// The command path below the tool's name, such as `books show`, or the
    /// tool's name for its root command.
    command: String,
    /// What the pattern suggests, or the figure that raised it.
    suggestion: String,
    /// The code of the failure that the runs retried, for a retry after
    /// failure only.
    code: Option<String>,
    /// The figures of the command since the start of the report.
    figures: CommandFigures,
}

impl crumbkit::Row for FindingRow {
    const COLUMNS: &'static [&'static str] = &["finding", "runs", "command", "suggestion"];
}

/// The figures of one command since the start of the report.
#[derive(Debug, Serialize, JsonSchema)]
pub(super) struct CommandFigures {
    /// The runs that did not print a help page.
    runs: u64,
    /// The runs that printed the command's help page.
    help_pages: u64,
    /// The help pages that each run's session read since its last run
    /// that printed no help page, summed over the runs.
    help_before: u64,
    /// The runs whose command line was refused.
    refused: u64,
    /// The runs that failed.
    failed: u64,
    /// The 95th percentile of the runs' wall time in milliseconds, or null
    /// when every run printed help.
    p95_ms: Option<u64>,
    /// The 95th percentile of the bytes that the runs wrote to standard
    /// output, or null when no run printed a result.
    p95_out_bytes: Option<u64>,
    /// The runs that typed `--full`.
    typed_full: u64,
    /// The runs that typed `--all`.
    typed_all: u64,
    /// The runs that typed `--long`.
    typed_long: u64,
    /// The runs that exited with each code from 3, which the tool declares,
    /// in code order.
    exits: Vec<ExitRuns>,
}

/// The runs of a command that exited with one of the tool's codes.
#[derive(Debug, Serialize, JsonSchema)]
struct ExitRuns {
    /// The exit code, from 3 to 255.
    exit: u8,
    /// What the tool's guide says the code means for the command, or null
    /// when the guide declares no such code for it.
    meaning: Option<&'static str>,
    /// The runs that exited with the code.
    runs: u64,
}

/// The most codes that the summary of `self usage` lists as tolerated.
const MOST_TOLERATED: usize = 10;

/// The summary of the runs that `self usage` read. The text output prints
/// it as one line above the table, such as `1,284 runs in 23 sessions since
/// 30 Sep: claude-code 1,201, person 83.`
#[derive(Debug, Serialize, JsonSchema)]
pub struct UsageSummary {
    /// The runs that the report read.
    runs: u64,
    /// The sessions that the runs came from.
    sessions: u64,
    /// The runs of each caller, the most first.
    callers: Vec<CallerRuns>,
    /// The day in UTC on which the report starts, such as 2026-09-30.
    since: String,
    /// The records that the report left out, because they named a command,
    /// flag, code or caller that the tool does not write.
    skipped: u64,
    /// The codes of the failures that runs reported and carried on past,
    /// with the runs that tolerated each, the most first, at most 10.
    tolerated: Vec<CodeRuns>,
}

/// The runs that tolerated one code.
#[derive(Debug, Serialize, JsonSchema)]
struct CodeRuns {
    /// The code, in the tool's spelling.
    code: String,
    /// The runs that tolerated the code. A run counts each code once.
    runs: u64,
}

/// The runs of one caller.
#[derive(Debug, Serialize, JsonSchema)]
struct CallerRuns {
    /// The coding agent's id, such as claude-code, person or unknown.
    caller: String,
    /// The caller's runs.
    runs: u64,
}

/// Returns the findings in the usage log of the tool whose command `output`
/// holds, which it builds in full through [`Output::command`], for the runs that ended in the `--since`
/// duration before now, for the tool to pass to [`finish`](crumbkit::finish)
/// with [`SELF_USAGE`]. `guide` is the tool's guide, whose exit codes give
/// the meaning of each exit from 3 in a command's figures.
///
/// It reads the file that `--log` names, or else `usage.jsonl` in the
/// tool's directory under the directory where [`start`](super::start)
/// writes it, once, a line at a time. A record that names a command path,
/// flag, failure code or caller that the tool's usage log could not have
/// written is left out and counted in the summary, so a hand-edited line
/// cannot put its text in the report. A code counts when it is 1 to 128
/// ASCII letters, digits, `_`, `-`, `:`, `.` and `/`, which covers the
/// predefined spellings and a custom one.
///
/// A failure to find or read the log stays silent, as every usage-log
/// failure does, and the report holds the runs read before it. A tool whose
/// root command has an empty name has no name to find its log by, so its
/// report has no runs.
#[must_use]
pub fn findings(
    output: &mut Output,
    guide: &Guide,
    args: &SelfUsage,
) -> Listing<FindingRow, UsageSummary> {
    let tool = output.command();
    report_from(tool, guide, open(tool, args), args.since, SystemTime::now())
}

/// Opens the usage log that `args` names, or else the log of `tool`, to
/// read it, or returns `None` when it cannot. A tool without a name has no
/// log, because the reader checks each record against the tool's commands.
pub(super) fn open(tool: &Command, args: &SelfUsage) -> Option<BufReader<File>> {
    if tool.get_name().is_empty() {
        return None;
    }
    let path = match &args.log {
        Some(path) => path.clone(),
        None => super::platform::state_home()?
            .join(tool.get_name())
            .join(LOG),
    };
    File::open(path).ok().map(BufReader::new)
}

/// The last second that humantime formats, 9999-12-31T23:59:59Z, in
/// seconds since the Unix epoch. humantime panics on a later time.
const LAST_SECOND: u64 = 253_402_300_799;

/// Returns the UTC day of `time` as `YYYY-MM-DD`, with a time after
/// [`LAST_SECOND`] read as that second and a time before 1970 read as the
/// epoch, so that a wrong clock cannot panic.
fn day_of(time: SystemTime) -> String {
    let time = time.clamp(UNIX_EPOCH, UNIX_EPOCH + Duration::from_secs(LAST_SECOND));
    let stamp = humantime::format_rfc3339_seconds(time).to_string();
    // The first 10 characters of an RFC 3339 time are its day.
    stamp.get(..10).unwrap_or_default().to_owned()
}

/// Returns the findings in `log` for the runs that ended in the `since`
/// duration before `now`, checked against `tool`, as [`findings`]
/// describes for `guide`.
fn report_from(
    tool: &Command,
    guide: &Guide,
    log: Option<impl BufRead>,
    since: Duration,
    now: SystemTime,
) -> Listing<FindingRow, UsageSummary> {
    let (report, summary) = summarize(tool, guide, log, since, now);
    let rows: Vec<FindingRow> = report
        .findings
        .iter()
        .map(|finding| row(tool.get_name(), guide, finding, &report))
        .collect();
    let total = rows.len();
    Listing::new(rows, total)
        .described("findings")
        .summary(summary)
        .next(
            Call::new(&["self", "usage"]).arg("json").arg("full"),
            "adds the figures of each finding's command.",
        )
}

/// Returns the report of the runs in `log` that ended in the `since`
/// duration before `now`, checked against `tool`, and its summary. A run
/// that exited with the code that `guide` gives a refused command line
/// counts as refused.
pub(super) fn summarize(
    tool: &Command,
    guide: &Guide,
    log: Option<impl BufRead>,
    since: Duration,
    now: SystemTime,
) -> (Report, UsageSummary) {
    // A start before 1970 reads from 1970, because no record is older and
    // humantime panics on a time before the epoch.
    let start = now.checked_sub(since).unwrap_or(UNIX_EPOCH).max(UNIX_EPOCH);
    let start_ms = start.duration_since(UNIX_EPOCH).map_or(0, |elapsed| {
        u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
    });
    let mut tally = Tally::new(guide.failure_exit(&[], &crumbkit::codes::CLI_USAGE));
    if let Some(log) = log {
        read(log, start_ms, tool, &mut tally);
    }
    let mut report = tally.report();
    let summary = UsageSummary {
        runs: report.runs,
        sessions: report.sessions,
        callers: std::mem::take(&mut report.callers)
            .into_iter()
            .map(|count| CallerRuns {
                caller: count.name,
                runs: count.runs,
            })
            .collect(),
        since: day_of(start),
        skipped: report.skipped,
        tolerated: std::mem::take(&mut report.tolerated)
            .into_iter()
            .take(MOST_TOLERATED)
            .map(|count| CodeRuns {
                code: count.name,
                runs: count.runs,
            })
            .collect(),
    };
    (report, summary)
}

/// Returns the row of `finding` in `report` for the tool named `tool`,
/// whose guide is `guide`.
fn row(tool: &str, guide: &Guide, finding: &Finding, report: &Report) -> FindingRow {
    // Every finding names a command that the report has figures for.
    let figures = report
        .commands
        .binary_search_by(|figures| figures.command.as_str().cmp(&finding.command))
        .ok()
        .and_then(|index| report.commands.get(index));
    let command = if finding.command.is_empty() {
        tool.to_owned()
    } else {
        finding.command.clone()
    };
    FindingRow {
        finding: finding.kind,
        runs: finding.runs,
        command,
        suggestion: suggestion(finding, figures),
        code: finding.code.clone(),
        figures: figures.map_or_else(CommandFigures::none, |figures| {
            CommandFigures::of(figures, guide)
        }),
    }
}

/// Returns the suggestion of `finding`, which reads the figures of its
/// command.
fn suggestion(finding: &Finding, figures: Option<&Figures>) -> String {
    match finding.kind {
        Kind::OneIdAtATime => "pass every id to one call".to_owned(),
        Kind::IdenticalRepeat => "the same call ran again within 60 s".to_owned(),
        Kind::HelpLoop => "3+ help pages before the first success".to_owned(),
        Kind::UsageError => match figures.and_then(|figures| figures.nearest.first()) {
            Some(nearest) => format!(
                "{} of the runs typed a flag close to {}",
                grouped(nearest.runs),
                nearest.name
            ),
            None => "the command line did not match the command's arguments".to_owned(),
        },
        Kind::RetryAfterFailure => format!(
            "{} failed and the command ran again within 60 s",
            finding.code.as_deref().unwrap_or("a failure")
        ),
        Kind::ShortPage => "a cut-short list ran again with --all or --limit".to_owned(),
        Kind::HeavyOutput => format!(
            "the 95th percentile of its output is {} bytes",
            grouped(
                figures
                    .and_then(|figures| figures.p95_out_bytes)
                    .unwrap_or(0)
            )
        ),
        Kind::SlowCommand => format!(
            "the 95th percentile of its wall time is {} ms",
            grouped(figures.and_then(|figures| figures.p95_ms).unwrap_or(0))
        ),
    }
}

impl CommandFigures {
    /// Returns the figures that `figures` holds, with the meaning that
    /// `guide` gives each exit code for the command.
    pub(super) fn of(figures: &Figures, guide: &Guide) -> Self {
        let path: Vec<&str> = if figures.command.is_empty() {
            Vec::new()
        } else {
            figures.command.split(' ').collect()
        };
        let exits = figures
            .exits
            .iter()
            .map(|&(exit, runs)| ExitRuns {
                exit,
                meaning: guide
                    .get_exits()
                    .iter()
                    .find(|declared| declared.codes().contains(&exit) && declared.covers(&path))
                    .map(crumbkit::help::Exit::meaning),
                runs,
            })
            .collect();
        Self {
            runs: figures.runs,
            help_pages: figures.help,
            help_before: figures.help_before,
            refused: figures.refused,
            failed: figures.failed,
            p95_ms: figures.p95_ms,
            p95_out_bytes: figures.p95_out_bytes,
            typed_full: figures.full,
            typed_all: figures.all,
            typed_long: figures.long,
            exits,
        }
    }

    /// Returns the figures of a command without runs.
    const fn none() -> Self {
        Self {
            runs: 0,
            help_pages: 0,
            help_before: 0,
            refused: 0,
            failed: 0,
            p95_ms: None,
            p95_out_bytes: None,
            typed_full: 0,
            typed_all: 0,
            typed_long: 0,
            exits: Vec::new(),
        }
    }
}

/// The abbreviated names of the months, which the summary line prints.
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

impl fmt::Display for UsageSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let plural = |count: u64, one: &'static str, many: &'static str| {
            if count == 1 { one } else { many }
        };
        write!(
            f,
            "{} {}",
            grouped(self.runs),
            plural(self.runs, "run", "runs")
        )?;
        if self.sessions > 0 {
            write!(
                f,
                " in {} {}",
                grouped(self.sessions),
                plural(self.sessions, "session", "sessions")
            )?;
        }
        write!(f, " since {}", day(&self.since))?;
        for (index, count) in self.callers.iter().enumerate() {
            let separator = if index == 0 { ":" } else { "," };
            write!(f, "{separator} {} {}", count.caller, grouped(count.runs))?;
        }
        f.write_str(".")?;
        if self.skipped > 0 {
            write!(
                f,
                " {} {} skipped because they name a command, flag, code or caller that the tool does not write.",
                grouped(self.skipped),
                plural(self.skipped, "record was", "records were")
            )?;
        }
        for (index, count) in self.tolerated.iter().enumerate() {
            let lead = if index == 0 {
                " Tolerated most often:"
            } else {
                ","
            };
            write!(f, "{lead} {} {}", count.code, grouped(count.runs))?;
        }
        if !self.tolerated.is_empty() {
            f.write_str(".")?;
        }
        Ok(())
    }
}

/// Returns the day of `date`, such as `2026-09-30`, as the summary line
/// prints it, such as `30 Sep`, or `date` itself when it is not in that
/// form.
fn day(date: &str) -> Cow<'_, str> {
    let month = date
        .get(5..7)
        .and_then(|month| month.parse::<usize>().ok())
        .and_then(|month| MONTHS.get(month.checked_sub(1)?));
    let day = date.get(8..10).map(|day| day.trim_start_matches('0'));
    match (month, day) {
        (Some(month), Some(day)) => Cow::Owned(format!("{day} {month}")),
        _ => Cow::Borrowed(date),
    }
}

/// Returns `count` with a comma between each group of three digits, such
/// as `1,284`.
fn grouped(count: u64) -> String {
    let digits = count.to_string();
    let mut text = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            text.push(',');
        }
        text.push(digit);
    }
    text
}
