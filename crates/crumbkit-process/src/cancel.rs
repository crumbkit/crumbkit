//! The handle that stops a run from another thread.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

use crate::notice::Notice;

/// A handle that stops the runs that watch it.
///
/// A run watches a cancel that
/// [`RunOptions::cancel`](crate::RunOptions::cancel) gave it.
/// [`Cancel::cancel`] stops each watching run with its escalation, and the
/// run ends as [`Stopped(Cancelled)`](crate::StopReason::Cancelled). A run that
/// starts with a cancel that is already cancelled starts no child. Clones
/// share one state, so any clone cancels, and a cancel stays cancelled.
#[derive(Clone, Debug, Default)]
pub struct Cancel(Arc<Mutex<Watched>>);

/// Whether a cancel was cancelled, and the runs that watch it.
#[derive(Debug, Default)]
struct Watched {
    cancelled: bool,
    runs: Vec<Weak<Notice>>,
}

impl Cancel {
    /// Returns a cancel that is not cancelled.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Cancels every run that watches this cancel or that starts watching it
    /// later.
    pub fn cancel(&self) {
        let mut watched = self.lock();
        watched.cancelled = true;
        for run in watched.runs.drain(..).filter_map(|run| run.upgrade()) {
            run.set(|flags| flags.cancelled = true);
        }
    }

    /// Returns true once [`Cancel::cancel`] has been called on this cancel or
    /// a clone of it.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.lock().cancelled
    }

    /// Makes the run that `notice` wakes watch this cancel, and sets its
    /// flag at once when this cancel is already cancelled. It forgets the
    /// runs that have ended.
    pub(crate) fn watch(&self, notice: &Arc<Notice>) {
        let mut watched = self.lock();
        if watched.cancelled {
            notice.set(|flags| flags.cancelled = true);
            return;
        }
        watched.runs.retain(|run| run.strong_count() > 0);
        watched.runs.push(Arc::downgrade(notice));
    }

    /// Returns the locked state. Each holder only reads or changes it in one
    /// step, so a panic while the lock was held leaves nothing half changed.
    fn lock(&self) -> MutexGuard<'_, Watched> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
