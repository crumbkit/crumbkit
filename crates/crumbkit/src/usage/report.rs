//! The reader of the usage log, which reduces its lines to the figures and
//! findings that `self usage` reports (ADR-0029).
//!
//! [`read`] reads the log once, line by line, and adds each record to a
//! [`Tally`], which keeps counts, the durations and output sizes of each
//! command and what each session's findings need of its earlier runs, and
//! never a copy of the file. [`Tally::report`] then gives the [`Report`].
//! A line that is not a whole `crumbkit/usage/v1` record is skipped, so a
//! damaged log reports the lines that can be read.

mod entry;
mod known;
mod session;
mod tally;

use std::io::{BufRead, Read as _};

use clap::Command;
use schemars::JsonSchema;
use serde::Serialize;

use self::entry::Entry;
pub(super) use self::tally::Tally;
use super::record::MAX_LINE;

#[cfg(test)]
mod tests;

/// The longest line that the reader reads, in bytes with its newline. The
/// usage log never writes a longer line, so a longer one is damaged, and
/// the reader skips it without holding more than this many bytes of it.
const LONGEST: usize = MAX_LINE + 1;

/// Adds to `tally` every record of `log` whose run ended at or after
/// `since`, in milliseconds since the Unix epoch, and that names only what
/// the writer could have produced for `tool`, as [`known::known`] checks.
/// `tool` is the tool's command after `Command::build`. A record that
/// names anything else is counted as skipped and adds nothing more.
///
/// It reads one line at a time and keeps at most [`LONGEST`] bytes of it.
/// A line that is not a whole record, such as the last line of a run that
/// ended while it wrote, is skipped. Reading stops at the first I/O error,
/// and the tally keeps the records read before it, because a usage-log
/// failure stays silent (ADR-0029).
pub(super) fn read(mut log: impl BufRead, since: u64, tool: &Command, tally: &mut Tally) {
    let mut line = Vec::with_capacity(LONGEST);
    loop {
        line.clear();
        match log
            .by_ref()
            .take(LONGEST as u64)
            .read_until(b'\n', &mut line)
        {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        if line.len() == LONGEST && line.last() != Some(&b'\n') {
            // The line is longer than any record, so the rest of it is
            // skipped unread.
            if log.skip_until(b'\n').is_err() {
                return;
            }
            continue;
        }
        let Some(entry) = Entry::parse(&line) else {
            continue;
        };
        if entry.at < since {
            continue;
        }
        if known::known(tool, &entry) {
            tally.add(&entry);
        } else {
            tally.skip();
        }
    }
}

/// What the usage log says about a tool's runs since a time.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Report {
    /// The number of runs.
    pub(super) runs: u64,
    /// The number of sessions that the runs came from.
    pub(super) sessions: u64,
    /// The number of records that named a command, flag, code or caller
    /// that the writer could not have produced, which the report leaves
    /// out.
    pub(super) skipped: u64,
    /// The runs of each caller, such as `claude-code`, the most first.
    pub(super) callers: Vec<Count>,
    /// The findings, the most runs first.
    pub(super) findings: Vec<Finding>,
    /// The figures of each command, in the order of their paths.
    pub(super) commands: Vec<Figures>,
}

/// A pattern in the runs of one command that suggests where callers waste
/// effort.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Finding {
    /// What the pattern is.
    pub(super) kind: Kind,
    /// The command path below the tool's name, or the empty string for the
    /// tool's root command.
    pub(super) command: String,
    /// The runs that show the pattern.
    pub(super) runs: u64,
    /// The code of the failure that the runs retried, for
    /// [`Kind::RetryAfterFailure`] only.
    pub(super) code: Option<String>,
}

/// The kinds of finding, with the runs that each one counts. Each kind
/// serializes as the name that `self usage` prints, such as
/// `one id at a time`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, JsonSchema)]
pub(super) enum Kind {
    /// A session ran a command that takes many ids with one id at least 3
    /// times within 2 minutes. It counts those runs.
    #[serde(rename = "one id at a time")]
    OneIdAtATime,
    /// A session ran the same command line again within 60 seconds. It
    /// counts the repeats.
    #[serde(rename = "identical repeat")]
    IdenticalRepeat,
    /// A session read at least 3 help pages before its first successful
    /// run, or without one. It counts the pages, under the command of the
    /// last page.
    #[serde(rename = "help loop")]
    HelpLoop,
    /// The parser refused the command line. It counts the refused runs.
    #[serde(rename = "usage error")]
    UsageError,
    /// A session ran a command again within 60 seconds of its failure. It
    /// counts the retries, under the code of the failure.
    #[serde(rename = "retry after failure")]
    RetryAfterFailure,
    /// A session followed a cut-short listing within 60 seconds with the
    /// same command and `--all` or `--limit`. It counts those runs. The
    /// record holds no limit value, so any `--limit` counts, also one that
    /// asks for fewer rows than the default page.
    #[serde(rename = "short page")]
    ShortPage,
    /// The 95th percentile of a command's output is at least 20,000 bytes.
    /// It counts the runs that wrote that much.
    #[serde(rename = "heavy output")]
    HeavyOutput,
    /// The 95th percentile of a command's wall time is at least 1 second.
    /// It counts the runs that took that long.
    #[serde(rename = "slow command")]
    SlowCommand,
}

/// A name and the number of runs that it counts.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Count {
    /// The name, such as a caller or a flag.
    pub(super) name: String,
    /// The number of runs.
    pub(super) runs: u64,
}

/// The figures of one command path.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Figures {
    /// The command path below the tool's name, such as `books show`, or the
    /// empty string for the tool's root command.
    pub(super) command: String,
    /// The runs that did not print a help page.
    pub(super) runs: u64,
    /// The runs that printed this command's help page.
    pub(super) help: u64,
    /// The runs whose command line was refused.
    pub(super) refused: u64,
    /// The flags that clap suggested for the refused runs, the most
    /// suggested first.
    pub(super) nearest: Vec<Count>,
    /// The runs that failed.
    pub(super) failed: u64,
    /// The 95th percentile of the runs' wall time in milliseconds, or `None`
    /// when every run printed help.
    pub(super) p95_ms: Option<u64>,
    /// The 95th percentile of the bytes that the runs wrote to standard
    /// output, or `None` when no run reached `finish`.
    pub(super) p95_out_bytes: Option<u64>,
    /// The runs that typed `--full`.
    pub(super) full: u64,
    /// The runs that typed `--all`.
    pub(super) all: u64,
    /// The runs that typed `--long`.
    pub(super) long: u64,
}
