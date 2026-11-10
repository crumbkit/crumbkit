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
///
/// Any thread may call [`Cancel::cancel`] at any time, before, during or
/// after a run. A cancel that arrives once the run has seen the child's exit
/// changes nothing, and the run gives the child's own end. When the run sees
/// a cancel and the exit together, the exit wins. Otherwise the run gives
/// `Stopped(Cancelled)`, which a cancel just after the exit can also give.
/// [`ChildFinished::status`](crate::ChildFinished::status) gives the child's
/// own status in each case.
///
/// The run does not poll a flag of the caller's. A tool whose stop request is a flag, such
/// as one that a signal handler sets, polls that flag on a thread of its
/// own and calls [`Cancel::cancel`] from there. A signal handler itself must
/// not call it, because it takes a lock. Such a tool decides what the stop
/// means from its own flag, because a status such as `Signal(2)` also
/// follows a signal that another process sent.
///
/// ```
/// use std::process::Command;
/// use std::sync::Arc;
/// use std::sync::atomic::{AtomicBool, Ordering};
/// use std::time::Duration;
///
/// use crumbkit_process::{Cancel, RunOptions, run};
///
/// fn run_until(flag: &Arc<AtomicBool>, command: Command) -> bool {
///     let cancel = Cancel::new();
///     let done = Arc::new(AtomicBool::new(false));
///     let poller = std::thread::spawn({
///         let (flag, cancel, done) = (Arc::clone(flag), cancel.clone(), Arc::clone(&done));
///         move || {
///             while !done.load(Ordering::Relaxed) {
///                 if flag.load(Ordering::Relaxed) {
///                     cancel.cancel();
///                     return;
///                 }
///                 std::thread::sleep(Duration::from_millis(10));
///             }
///         }
///     });
///     let finished = run(command, &RunOptions::new().cancel(&cancel));
///     done.store(true, Ordering::Relaxed);
///     let _ = poller.join();
///     finished.is_ok()
/// }
/// ```
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
