//! What wakes a run's own thread: the child's exit, a cancel, and an output
//! stream that passed its limit.

use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

/// The events that a run's thread waits for, each set once.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Flags {
    /// The child has exited and is still unreaped.
    pub(crate) exited: bool,
    /// The run's cancel was cancelled.
    pub(crate) cancelled: bool,
    /// An output stream whose policy stops the child passed its limit.
    pub(crate) over_limit: bool,
}

/// The flags of one run and the condition variable that wakes its thread
/// when one is set.
#[derive(Debug, Default)]
pub(crate) struct Notice {
    flags: Mutex<Flags>,
    changed: Condvar,
}

impl Notice {
    /// Sets flags with `set` and wakes the run's thread.
    pub(crate) fn set(&self, set: impl FnOnce(&mut Flags)) {
        set(&mut self.lock());
        self.changed.notify_all();
    }

    /// Waits until `done` holds for the flags or `until` passes, and returns
    /// the flags then. With no `until` it waits until `done` holds.
    pub(crate) fn wait(&self, until: Option<Instant>, done: impl Fn(&Flags) -> bool) -> Flags {
        let mut flags = self.lock();
        loop {
            if done(&flags) {
                return *flags;
            }
            flags = match until {
                None => self
                    .changed
                    .wait(flags)
                    .unwrap_or_else(PoisonError::into_inner),
                Some(until) => {
                    let left = until.saturating_duration_since(Instant::now());
                    if left.is_zero() {
                        return *flags;
                    }
                    self.changed
                        .wait_timeout(flags, left)
                        .unwrap_or_else(PoisonError::into_inner)
                        .0
                }
            };
        }
    }

    /// Returns the locked flags. Each holder only reads or sets a flag, so a
    /// panic while the lock was held leaves nothing half changed.
    fn lock(&self) -> MutexGuard<'_, Flags> {
        self.flags.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
