//! The bounded queue that carries the lines of one output stream from the
//! run's reader to the caller while the run goes on.
//!
//! A full queue makes the reader wait, and the child then waits on its full
//! pipe. Once the run begins to stop the child, a line that finds the queue
//! full is dropped and kept as the stream's last line, so a caller that
//! stopped reading cannot hold up the stop. Once the caller drops its
//! [`Lines`], the reader reads the rest of the stream and drops it.

use std::collections::VecDeque;
use std::fmt;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

#[cfg(test)]
mod tests;

/// Returns a queue that holds at most `most` lines, or one line when
/// `most` is 0: the side that a run fills, which
/// [`Capture::Lines`](crate::Capture::Lines) takes, and the caller's side.
///
/// Each queue serves one run. The run ends the queue when it returns, on
/// every path, including a start failure and a cancel before the start, so
/// [`Lines::recv`] then returns `None` once the queue is empty.
#[must_use]
pub fn line_queue(most: usize) -> (LineQueue, Lines) {
    let shared = Arc::new(Shared {
        most: most.max(1),
        state: Mutex::new(State::default()),
        changed: Condvar::new(),
    });
    (LineQueue(Arc::clone(&shared)), Lines(shared))
}

/// What the run's reader and the caller share.
#[derive(Debug)]
struct Shared {
    most: usize,
    state: Mutex<State>,
    changed: Condvar,
}

/// The queued lines and what has happened to the run and the caller.
#[derive(Debug, Default)]
struct State {
    lines: VecDeque<Vec<u8>>,
    /// The last line that a full queue dropped, while no line has been
    /// queued after it. It is the stream's real last line.
    lost: Option<Vec<u8>>,
    stopping: bool,
    ended: bool,
    abandoned: bool,
}

/// The side of a line queue that a run fills. A run takes it in
/// [`Capture::Lines`](crate::Capture::Lines).
#[derive(Clone)]
pub struct LineQueue(Arc<Shared>);

impl fmt::Debug for LineQueue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LineQueue")
            .field("most", &self.0.most)
            .finish_non_exhaustive()
    }
}

impl LineQueue {
    /// Adds `line`, and waits while the queue is full, the run is not
    /// stopping and the caller still holds its [`Lines`]. A full queue
    /// after the run began to stop drops the line and keeps it as the
    /// stream's last line. Returns false when the caller has dropped its
    /// side, and then nobody reads the line.
    pub(crate) fn push(&self, line: Vec<u8>) -> bool {
        let shared = &*self.0;
        let mut state = shared.lock();
        while state.lines.len() >= shared.most && !state.stopping && !state.abandoned {
            state = shared
                .changed
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
        }
        if state.abandoned {
            return false;
        }
        if state.lines.len() < shared.most {
            state.lines.push_back(line);
            state.lost = None;
            shared.changed.notify_all();
        } else {
            state.lost = Some(line);
        }
        true
    }

    /// Marks that the run began to stop its child, so a full queue drops
    /// a line instead of waiting.
    pub(crate) fn stopping(&self) {
        self.0.lock().stopping = true;
        self.0.changed.notify_all();
    }

    /// Marks that the run has ended, so the caller's waits end once the
    /// queue is empty.
    pub(crate) fn end(&self) {
        self.0.lock().ended = true;
        self.0.changed.notify_all();
    }
}

/// The caller's side of a line queue, which yields each line of the stream
/// without its newline as the run reads it. Dropping it tells the run that
/// nobody reads the lines, and the run reads the rest of the stream and
/// drops it.
pub struct Lines(Arc<Shared>);

impl fmt::Debug for Lines {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Lines")
            .field("most", &self.0.most)
            .finish_non_exhaustive()
    }
}

/// No line arrived and the run did not end within the time that
/// [`Lines::recv_timeout`] waited.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Silent;

impl fmt::Display for Silent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("no line arrived in time")
    }
}

impl std::error::Error for Silent {}

impl Lines {
    /// Waits for the next line and returns it, or returns `None` once the
    /// run has ended and every queued line has been returned.
    #[must_use]
    pub fn recv(&self) -> Option<Vec<u8>> {
        let shared = &*self.0;
        let mut state = shared.lock();
        while state.lines.is_empty() && !state.ended {
            state = shared
                .changed
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
        }
        shared.take(&mut state)
    }

    /// Waits at most `timeout` for the next line, as [`Lines::recv`] does.
    ///
    /// # Errors
    ///
    /// Returns [`Silent`] when no line arrived and the run did not end
    /// within `timeout`.
    pub fn recv_timeout(&self, timeout: Duration) -> Result<Option<Vec<u8>>, Silent> {
        let shared = &*self.0;
        // A timeout too long for an `Instant` waits without a limit.
        let deadline = Instant::now().checked_add(timeout);
        let mut state = shared.lock();
        while state.lines.is_empty() && !state.ended {
            let left = deadline.map_or(timeout, |deadline| {
                deadline.saturating_duration_since(Instant::now())
            });
            if left.is_zero() {
                return Err(Silent);
            }
            state = shared
                .changed
                .wait_timeout(state, left)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
        Ok(shared.take(&mut state))
    }

    /// Returns the stream's last line when a full queue dropped it after
    /// the run began to stop, so [`Lines::recv`] never returned it. It
    /// returns `None` until the run has ended, and the line only once.
    #[must_use]
    pub fn lost(&self) -> Option<Vec<u8>> {
        let mut state = self.0.lock();
        if state.ended { state.lost.take() } else { None }
    }
}

impl Drop for Lines {
    /// Tells the run that nobody reads the lines, so its reader stops
    /// waiting for room.
    fn drop(&mut self) {
        self.0.lock().abandoned = true;
        self.0.changed.notify_all();
    }
}

impl Shared {
    /// Takes the next line from `state`, and wakes a reader that waits for
    /// room.
    fn take(&self, state: &mut State) -> Option<Vec<u8>> {
        let line = state.lines.pop_front();
        if line.is_some() {
            self.changed.notify_all();
        }
        line
    }

    /// Returns the locked state. Each holder changes it in one step, so a
    /// panic while the lock was held leaves nothing half changed.
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
