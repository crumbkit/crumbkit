//! The running reduction of the usage log's records into counts.

use std::collections::{BTreeMap, HashSet};

use super::entry::Entry;
use super::{Count, Figures, Report};

#[cfg(test)]
mod tests;

/// The counts of the records read so far.
///
/// Its memory grows with the number of records, by two numbers for each
/// run of a command, and never holds a record's line.
#[derive(Debug, Default)]
pub(in crate::usage) struct Tally {
    /// The number of runs.
    runs: u64,
    /// The session hashes seen.
    sessions: HashSet<String>,
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
        if !self.sessions.contains(&entry.session) {
            self.sessions.insert(entry.session.clone());
        }
        *self.callers.entry(entry.caller.clone()).or_default() += 1;
        let path = entry.command.clone().unwrap_or_default();
        let command = self.commands.entry(path).or_default();
        if entry.help {
            command.help += 1;
            return;
        }
        command.runs += 1;
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

    /// Returns the report of every record added.
    pub(in crate::usage) fn report(self) -> Report {
        let commands = self
            .commands
            .into_iter()
            .map(|(path, mut command)| Figures {
                command: path,
                runs: command.runs,
                help: command.help,
                refused: command.refused,
                nearest: most_first(command.nearest),
                failed: command.failed,
                p95_ms: p95(&mut command.ms),
                p95_out_bytes: p95(&mut command.out_bytes),
                full: command.full,
                all: command.all,
                long: command.long,
            })
            .collect();
        Report {
            runs: self.runs,
            sessions: self.sessions.len() as u64,
            callers: most_first(self.callers),
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
