//! The report of a run whose child is still running at a chosen time.

use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// What a run tells [`RunOptions::overdue`](crate::RunOptions::overdue)'s
/// report: the child is still running at the overdue time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Overdue {
    id: u32,
    elapsed: Duration,
}

impl Overdue {
    pub(crate) const fn new(id: u32, elapsed: Duration) -> Self {
        Self { id, elapsed }
    }

    /// Returns the child's process id. The child is unreaped until the run
    /// ends, so the id names it and no other process.
    #[must_use]
    pub const fn id(&self) -> u32 {
        self.id
    }

    /// Returns the time from the child's start to the report.
    #[must_use]
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }
}

/// The caller's report and the time it runs at.
#[derive(Clone)]
pub(super) struct Report {
    pub(super) at: Instant,
    report: Arc<dyn Fn(Overdue) + Send + Sync>,
}

impl Report {
    /// Wraps the caller's function.
    pub(super) fn new(at: Instant, report: impl Fn(Overdue) + Send + Sync + 'static) -> Self {
        Self {
            at,
            report: Arc::new(report),
        }
    }

    /// Calls the caller's function.
    pub(super) fn call(&self, overdue: Overdue) {
        (self.report)(overdue);
    }
}

impl fmt::Debug for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Report")
            .field("at", &self.at)
            .finish_non_exhaustive()
    }
}
