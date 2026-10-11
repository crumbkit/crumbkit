//! The run time check: the median wall time of a whole run of the release
//! build of shelf has a budget of 10 ms and a limit of 30 ms (PERF-2). A
//! median over the budget prints a warning and passes, and a median over the
//! limit fails.

use std::fmt;
use std::time::Duration;

#[cfg(test)]
mod tests;

/// The shelf commands that the check times, as the arguments after `shelf`.
/// `books list` is the smallest whole run, and `commands list --all --json
/// --full` walks the whole command tree.
pub(crate) const COMMANDS: [&[&str]; 2] = [
    &["books", "list"],
    &["commands", "list", "--all", "--json", "--full"],
];

/// How many untimed runs of each command come first, so the operating system
/// has the binary and its libraries in memory before the timed runs.
pub(crate) const WARM_UP_RUNS: usize = 20;

/// How many timed runs of each command the median covers.
pub(crate) const TIMED_RUNS: usize = 200;

/// The largest median that is within the budget.
const BUDGET: Duration = Duration::from_millis(10);

/// The largest median that passes.
const LIMIT: Duration = Duration::from_millis(30);

/// The timing of one shelf command.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Timing {
    /// The command's arguments after `shelf`, joined with spaces.
    pub(crate) command: String,
    /// How many timed runs the timing covers.
    pub(crate) runs: usize,
    /// The median wall time of the runs. For an even number of runs it is
    /// the mean of the middle two.
    pub(crate) median: Duration,
    /// The 95th percentile wall time of the runs, by the nearest-rank method:
    /// the smallest run time that at least 95 percent of the runs do not
    /// exceed.
    pub(crate) p95: Duration,
}

impl Timing {
    /// Returns whether the timing has runs and its median is at most the
    /// limit. A timing without runs fails, because a missing measurement
    /// would otherwise pass silently.
    pub(crate) fn passes(&self) -> bool {
        self.runs > 0 && self.median <= LIMIT
    }

    /// Returns whether the median is over the budget.
    pub(crate) fn over_budget(&self) -> bool {
        self.median > BUDGET
    }
}

/// Returns the timing of `command` from the wall time of each run in `times`.
pub(crate) fn measure(command: &str, times: &[Duration]) -> Timing {
    let mut sorted = times.to_vec();
    sorted.sort_unstable();
    let middle = sorted.len() / 2;
    let median = match sorted.len() {
        0 => Duration::ZERO,
        len if len.is_multiple_of(2) => (sorted[middle - 1] + sorted[middle]) / 2,
        _ => sorted[middle],
    };
    // The nearest rank of the 95th percentile is 95 percent of the run count,
    // rounded up, and ranks count from 1.
    let p95 = match sorted.len() {
        0 => Duration::ZERO,
        len => sorted[(len * 95).div_ceil(100) - 1],
    };
    Timing {
        command: command.to_owned(),
        runs: sorted.len(),
        median,
        p95,
    }
}

/// Prints the timing as one line that starts with the command, such as
/// `shelf books list: 3.2 ms median and 4.0 ms 95th percentile wall time of
/// 200 runs, budget 10 ms, limit 30 ms`.
impl fmt::Display for Timing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let command = &self.command;
        let limit = LIMIT.as_millis();
        if self.runs == 0 {
            return write!(f, "shelf {command}: no runs to measure, limit {limit} ms");
        }
        let median = self.median.as_secs_f64() * 1000.0;
        let p95 = self.p95.as_secs_f64() * 1000.0;
        let runs = self.runs;
        let budget = BUDGET.as_millis();
        let budget = if self.over_budget() {
            format!("over the budget of {budget} ms")
        } else {
            format!("budget {budget} ms")
        };
        write!(
            f,
            "shelf {command}: {median:.1} ms median and {p95:.1} ms 95th percentile wall time of {runs} runs, {budget}, limit {limit} ms"
        )
    }
}
