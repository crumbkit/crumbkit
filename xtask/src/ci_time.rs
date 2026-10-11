//! The CI time check: the median wall time of recent successful pull-request
//! runs of `.github/workflows/ci.yml` may be at most 180 seconds (CI-3).

use std::fmt;

#[cfg(test)]
mod tests;

/// The file name of the workflow whose runs the check measures, which the
/// GitHub API accepts as the workflow's ID.
pub(crate) const WORKFLOW_FILE: &str = "ci.yml";

/// How many of the most recent successful pull-request runs the median covers.
pub(crate) const RUNS: usize = 20;

/// The largest median wall time, in seconds, that passes.
const LIMIT_SECONDS: u64 = 180;

/// The outcome of measuring the recent runs.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Report {
    /// GitHub returned no successful pull-request runs, so there is nothing to
    /// measure. The check fails, because a missing measurement would otherwise
    /// pass silently.
    NoRuns,
    /// The median wall time of the measured runs.
    Median {
        /// How many runs were measured.
        runs: usize,
        /// The median wall time in whole seconds. For an even number of runs
        /// it is the mean of the middle two, rounded up, so a median between
        /// 180 and 181 seconds fails.
        seconds: u64,
    },
}

impl Report {
    /// Returns whether the median is at most the limit.
    pub(crate) fn passes(&self) -> bool {
        match self {
            Self::NoRuns => false,
            Self::Median { seconds, .. } => *seconds <= LIMIT_SECONDS,
        }
    }
}

/// Returns the median of the wall times in `seconds`, one per run.
pub(crate) fn measure(seconds: &[u64]) -> Report {
    if seconds.is_empty() {
        return Report::NoRuns;
    }
    let mut sorted = seconds.to_vec();
    sorted.sort_unstable();
    let middle = sorted.len() / 2;
    let median = if sorted.len().is_multiple_of(2) {
        (sorted[middle - 1] + sorted[middle]).div_ceil(2)
    } else {
        sorted[middle]
    };
    Report::Median {
        runs: sorted.len(),
        seconds: median,
    }
}

/// Prints the report as one line that starts with the workflow path, such as
/// `.github/workflows/ci.yml: 65 seconds median wall time of 20 pull-request
/// runs, limit 180`.
impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoRuns => write!(
                f,
                ".github/workflows/{WORKFLOW_FILE}: no successful pull-request runs to measure, limit {LIMIT_SECONDS} seconds"
            ),
            Self::Median { runs, seconds } => write!(
                f,
                ".github/workflows/{WORKFLOW_FILE}: {seconds} seconds median wall time of {runs} pull-request runs, limit {LIMIT_SECONDS}"
            ),
        }
    }
}
