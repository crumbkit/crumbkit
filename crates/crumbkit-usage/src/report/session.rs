//! The findings that need a session's earlier runs: calls with one id at a
//! time, identical repeats, help loops, retries after a failure and listings
//! that were cut short.

use std::collections::{BTreeMap, HashMap, VecDeque};

use super::entry::Entry;
use super::{Finding, Kind};

#[cfg(test)]
mod tests;

/// How close together, in milliseconds, three single-id runs of one command
/// must end to count as one id at a time.
const ONE_ID_WINDOW: u64 = 120_000;

/// How many single-id runs of one command within [`ONE_ID_WINDOW`] count as
/// one id at a time.
const ONE_ID_RUNS: usize = 3;

/// How soon, in milliseconds, a run must follow an earlier run to count as
/// its identical repeat, its retry after a failure, or the next page of its
/// cut-short listing.
const FOLLOW_WINDOW: u64 = 60_000;

/// How many help pages a session reads before its first successful run to
/// count as a help loop.
const HELP_PAGES: u64 = 3;

/// The runs that each finding counts so far, by the finding's kind, its
/// command path and the code of the failure that a retry followed.
#[derive(Debug, Default)]
pub(super) struct Found(BTreeMap<(Kind, String, Option<String>), u64>);

impl Found {
    /// Adds `runs` to the finding of `kind` for `command` and `code`.
    fn add(&mut self, kind: Kind, command: &str, code: Option<&str>, runs: u64) {
        let key = (kind, command.to_owned(), code.map(str::to_owned));
        *self.0.entry(key).or_default() += runs;
    }

    /// Returns each finding with its runs, in the order of its key.
    pub(super) fn into_findings(self) -> impl Iterator<Item = Finding> {
        self.0
            .into_iter()
            .map(|((kind, command, code), runs)| Finding {
                kind,
                command,
                runs,
                code,
            })
    }
}

/// What the findings need to know about one session's earlier runs.
///
/// Its memory grows with the number of distinct command lines that the
/// session ran, and holds no command line, only its hash.
#[derive(Debug, Default)]
pub(super) struct Session {
    /// The single-id runs of each command path that ended within
    /// [`ONE_ID_WINDOW`] of the latest one, oldest first.
    single_ids: HashMap<String, VecDeque<Single>>,
    /// When the last run of each command-line hash ended.
    last_args: HashMap<String, u64>,
    /// The end time and code of each command path's last run, when that run
    /// failed.
    failures: HashMap<String, (u64, String)>,
    /// The end time of each command path's last run, when that run showed a
    /// cut-short listing.
    truncated: HashMap<String, u64>,
    /// The help pages that the session read before its first successful
    /// run.
    help_pages: u64,
    /// The command path of the last of those help pages.
    help_command: String,
    /// Whether the session has had a successful run.
    succeeded: bool,
    /// The help pages that the session read since its last run that printed
    /// no help page.
    help_since_run: u64,
}

/// A single-id run of a command.
#[derive(Debug)]
struct Single {
    /// When the run ended, in milliseconds since the Unix epoch.
    at: u64,
    /// Whether a finding already counts the run.
    counted: bool,
}

impl Session {
    /// Adds to `found` what `entry`, the session's next run in the order of
    /// end times, shows with the session's earlier runs.
    pub(super) fn add(&mut self, entry: &Entry, found: &mut Found) {
        let command = entry.command.as_deref().unwrap_or_default();
        self.repeat(entry, command, found);
        if entry.help {
            if !self.succeeded {
                self.help_pages += 1;
                command.clone_into(&mut self.help_command);
            }
            return;
        }
        if entry.ids == Some(1) {
            self.single_id(entry, command, found);
        }
        if let Some((at, code)) = self.failures.remove(command)
            && follows(at, entry.at)
        {
            found.add(Kind::RetryAfterFailure, command, Some(&code), 1);
        }
        if let Some(code) = entry.failure() {
            self.failures
                .insert(command.to_owned(), (entry.at, code.to_owned()));
        }
        if let Some(at) = self.truncated.remove(command)
            && follows(at, entry.at)
            && (entry.typed("--all") || entry.typed("--limit"))
        {
            found.add(Kind::ShortPage, command, None, 1);
        }
        if entry.truncated {
            self.truncated.insert(command.to_owned(), entry.at);
        }
        if entry.succeeded() && !self.succeeded {
            self.succeeded = true;
            self.help_loop(found);
        }
    }

    /// Returns the help pages that the session read since its last run
    /// that printed no help page, when `entry` prints none, which the tally
    /// credits to `entry`'s command. When `entry` prints a help page, it
    /// counts that page and returns 0.
    pub(super) fn help_before(&mut self, entry: &Entry) -> u64 {
        if entry.help {
            self.help_since_run += 1;
            0
        } else {
            std::mem::take(&mut self.help_since_run)
        }
    }

    /// Adds to `found` the help loop of a session that read its help pages
    /// and never had a successful run. The tally calls it once, when it
    /// reports.
    pub(super) fn end(&self, found: &mut Found) {
        if !self.succeeded {
            self.help_loop(found);
        }
    }

    /// Adds the help pages that the session read before its first
    /// successful run, or before its end, to `found` when there are at least
    /// [`HELP_PAGES`] of them.
    fn help_loop(&self, found: &mut Found) {
        if self.help_pages >= HELP_PAGES {
            found.add(Kind::HelpLoop, &self.help_command, None, self.help_pages);
        }
    }

    /// Adds `entry` to `found` as an identical repeat when the session ran
    /// the same command line within [`FOLLOW_WINDOW`] before it.
    fn repeat(&mut self, entry: &Entry, command: &str, found: &mut Found) {
        let previous = if let Some(at) = self.last_args.get_mut(&entry.args) {
            Some(std::mem::replace(at, entry.at))
        } else {
            self.last_args.insert(entry.args.clone(), entry.at);
            None
        };
        if previous.is_some_and(|at| follows(at, entry.at)) {
            found.add(Kind::IdenticalRepeat, command, None, 1);
        }
    }

    /// Adds `entry`, a run with one id, to the session's single-id runs of
    /// `command`, and counts the runs that are not counted yet when at least
    /// [`ONE_ID_RUNS`] of them ended within [`ONE_ID_WINDOW`].
    fn single_id(&mut self, entry: &Entry, command: &str, found: &mut Found) {
        let runs = self.single_ids.entry(command.to_owned()).or_default();
        while runs
            .front()
            .is_some_and(|first| entry.at.saturating_sub(first.at) > ONE_ID_WINDOW)
        {
            runs.pop_front();
        }
        runs.push_back(Single {
            at: entry.at,
            counted: false,
        });
        if runs.len() >= ONE_ID_RUNS {
            let mut new = 0;
            for run in runs.iter_mut().filter(|run| !run.counted) {
                run.counted = true;
                new += 1;
            }
            found.add(Kind::OneIdAtATime, command, None, new);
        }
    }
}

/// Returns whether a run that ended at `later` follows one that ended at
/// `earlier` within [`FOLLOW_WINDOW`]. Two runs of one session can end in
/// either order when they ran at the same time, and a later run that ended
/// first counts as following at once.
const fn follows(earlier: u64, later: u64) -> bool {
    later.saturating_sub(earlier) <= FOLLOW_WINDOW
}
