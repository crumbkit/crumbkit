//! The running reduction of the usage log's records into counts and
//! findings.

use std::collections::{BTreeMap, HashMap};

use super::entry::Entry;
use super::session::{Found, Session};
use super::{Count, Figures, Finding, Kind, Report};

#[cfg(test)]
mod tests;

/// The 95th percentile of a command's output, in bytes, at or above which
/// the command is a heavy-output finding. 20,000 bytes are about 5,000
/// tokens of a coding agent's context.
const HEAVY_OUTPUT: u64 = 20_000;

/// The 95th percentile of a command's wall time, in milliseconds, at or
/// above which the command is a slow-command finding.
const SLOW: u64 = 1_000;

/// The counts of the records read so far.
///
/// Its memory grows with the number of records, by two numbers for each
/// run of a command and what each session keeps of its earlier runs, and
/// never holds a record's line.
#[derive(Debug, Default)]
pub(in crate::usage) struct Tally {
    /// The number of runs.
    runs: u64,
    /// The number of records that the reader skipped.
    skipped: u64,
    /// What each session's findings need of its earlier runs, by the
    /// session's hash.
    sessions: HashMap<String, Session>,
    /// The runs that each finding of a session counts so far.
    found: Found,
    /// The runs of each caller.
    callers: BTreeMap<String, u64>,
    /// What each command path's runs did.
    commands: BTreeMap<String, Command>,
}

/// What the runs of one command path did.
#[derive(Debug, Default)]
struct Command {
    /// The runs that did not print a help page.
    runs: u64,
    /// The runs that printed a help page.
    help: u64,
    /// The help pages that the runs' sessions read since their last run
    /// that printed no help page.
    help_before: u64,
    /// The refused runs.
    refused: u64,
    /// The refused runs for each flag that clap suggested.
    nearest: BTreeMap<String, u64>,
    /// The failed runs.
    failed: u64,
    /// The wall time of each run that did not print a help page.
    ms: Vec<u64>,
    /// The bytes that each run that reached `finish` wrote.
    out_bytes: Vec<u64>,
    /// The runs that typed `--full`.
    full: u64,
    /// The runs that typed `--all`.
    all: u64,
    /// The runs that typed `--long`.
    long: u64,
}

impl Tally {
    /// Adds the run that `entry` records.
    pub(super) fn add(&mut self, entry: &Entry) {
        self.runs += 1;
        let help_before = if let Some(session) = self.sessions.get_mut(&entry.session) {
            session.add(entry, &mut self.found);
            session.help_before(entry)
        } else {
            let mut session = Session::default();
            session.add(entry, &mut self.found);
            let help_before = session.help_before(entry);
            self.sessions.insert(entry.session.clone(), session);
            help_before
        };
        *self.callers.entry(entry.caller.clone()).or_default() += 1;
        let path = entry.command.clone().unwrap_or_default();
        let command = self.commands.entry(path).or_default();
        if entry.help {
            command.help += 1;
            return;
        }
        command.runs += 1;
        command.help_before += help_before;
        command.ms.push(entry.ms);
        if let Some(bytes) = entry.out_bytes {
            command.out_bytes.push(bytes);
        }
        if entry.refused() {
            command.refused += 1;
            if let Some(flag) = &entry.nearest {
                *command.nearest.entry(flag.clone()).or_default() += 1;
            }
        }
        if entry.failure().is_some() {
            command.failed += 1;
        }
        command.full += u64::from(entry.typed("--full"));
        command.all += u64::from(entry.typed("--all"));
        command.long += u64::from(entry.typed("--long"));
    }

    /// Counts a record that the reader skipped.
    pub(super) const fn skip(&mut self) {
        self.skipped += 1;
    }

    /// Returns the report of every record added.
    pub(in crate::usage) fn report(mut self) -> Report {
        for session in self.sessions.values() {
            session.end(&mut self.found);
        }
        let mut findings: Vec<Finding> = self.found.into_findings().collect();
        let mut commands = Vec::with_capacity(self.commands.len());
        for (path, mut command) in self.commands {
            // `p95` sorts the values, which the findings then count from.
            let p95_ms = p95(&mut command.ms);
            let p95_out_bytes = p95(&mut command.out_bytes);
            let at_least = |values: &[u64], limit: u64| {
                (values.len() - values.partition_point(|value| *value < limit)) as u64
            };
            let mut finding = |kind, runs| {
                findings.push(Finding {
                    kind,
                    command: path.clone(),
                    runs,
                    code: None,
                });
            };
            if command.refused > 0 {
                finding(Kind::UsageError, command.refused);
            }
            if p95_out_bytes.is_some_and(|bytes| bytes >= HEAVY_OUTPUT) {
                finding(
                    Kind::HeavyOutput,
                    at_least(&command.out_bytes, HEAVY_OUTPUT),
                );
            }
            if p95_ms.is_some_and(|ms| ms >= SLOW) {
                finding(Kind::SlowCommand, at_least(&command.ms, SLOW));
            }
            commands.push(Figures {
                command: path,
                runs: command.runs,
                help: command.help,
                help_before: command.help_before,
                refused: command.refused,
                nearest: most_first(command.nearest),
                failed: command.failed,
                p95_ms,
                p95_out_bytes,
                full: command.full,
                all: command.all,
                long: command.long,
            });
        }
        findings.sort_by(|a, b| {
            b.runs
                .cmp(&a.runs)
                .then(a.kind.cmp(&b.kind))
                .then_with(|| a.command.cmp(&b.command))
                .then_with(|| a.code.cmp(&b.code))
        });
        Report {
            runs: self.runs,
            sessions: self.sessions.len() as u64,
            skipped: self.skipped,
            callers: most_first(self.callers),
            findings,
            commands,
        }
    }
}

/// Returns the names and counts of `counts`, the largest count first, and
/// names with equal counts in name order.
fn most_first(counts: BTreeMap<String, u64>) -> Vec<Count> {
    let mut counts: Vec<Count> = counts
        .into_iter()
        .map(|(name, runs)| Count { name, runs })
        .collect();
    // The sort is stable, so equal counts keep the map's name order.
    counts.sort_by_key(|count| std::cmp::Reverse(count.runs));
    counts
}

/// Returns the 95th percentile of `values` by the nearest-rank method: the
/// smallest value that at least 95 percent of the values are no larger
/// than. It sorts `values` in place, and returns `None` when it is empty.
fn p95(values: &mut [u64]) -> Option<u64> {
    values.sort_unstable();
    let rank = (values.len() * 95).div_ceil(100);
    values.get(rank.checked_sub(1)?).copied()
}
