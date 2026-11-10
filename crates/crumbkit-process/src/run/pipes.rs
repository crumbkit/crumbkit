//! The threads that read the child's output pipes and write its input, and
//! the time at which they stop waiting on a pipe.

use std::io::{self, Read, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use super::output::{Captured, Keeper, OutputCopy};
use crate::sys::{self, Pipe};

/// How long a read or a write waits on a pipe before it checks again
/// whether the run's drain has ended.
///
/// A second pipe whose writer the run closes could wake the wait at once,
/// but std on macOS creates a pipe and sets close-on-exec in two steps, so
/// a child that another thread starts in between would hold that writer
/// open.
const CHECK: Duration = Duration::from_millis(100);

/// The size of each read from a pipe.
const CHUNK: usize = 8 * 1024;

/// The time at which the run's readers and writer stop waiting on their
/// pipes, which the run sets once it has reaped the child.
#[derive(Debug, Default)]
pub(super) struct Ends(OnceLock<Instant>);

impl Ends {
    /// Makes every wait on a pipe end at `at`, unless a time was set
    /// before, which then stands.
    pub(super) fn set(&self, at: Instant) {
        let _ = self.0.set(at);
    }

    /// Returns the time at which waits end, once it is set.
    fn at(&self) -> Option<Instant> {
        self.0.get().copied()
    }

    /// Waits until `pipe` can be read, or written when `write` is true, and
    /// returns true, or returns false once the waits end.
    fn ready(&self, pipe: impl Pipe + Copy, write: bool) -> io::Result<bool> {
        loop {
            let wait = match self.at() {
                None => CHECK,
                Some(at) => {
                    let left = at.saturating_duration_since(Instant::now());
                    if left.is_zero() {
                        return Ok(false);
                    }
                    left.min(CHECK)
                }
            };
            if sys::ready(pipe, write, wait)? {
                return Ok(true);
            }
        }
    }
}

/// What the reader of one output stream kept, and when its first byte
/// arrived.
#[derive(Debug)]
pub(super) struct Kept {
    pub(super) captured: Captured,
    pub(super) first: Option<Instant>,
}

/// Reads `pipe` until it ends or the waits of `ends` end, keeps its bytes
/// with `keeper`, and calls `over` when the keeper asks to stop the child.
/// It keeps reading after that, so the child never waits on a full pipe
/// while it is stopped.
pub(super) fn read<P>(
    mut pipe: P,
    ends: &Ends,
    mut keeper: Keeper,
    copy: Option<&OutputCopy>,
    over: impl Fn(),
) -> io::Result<Kept>
where
    P: Read + Pipe,
{
    let mut first = None;
    let mut chunk = [0; CHUNK];
    let mut copy = copy;
    let mut panicked = false;
    while ends.ready(&pipe, false)? {
        let read = match pipe.read(&mut chunk) {
            Ok(0) => break,
            Ok(read) => read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        first.get_or_insert_with(Instant::now);
        if let Some(out) = copy
            && catch_unwind(AssertUnwindSafe(|| out.send(&chunk[..read]))).is_err()
        {
            copy = None;
            panicked = true;
        }
        if keeper.add(&chunk[..read]) {
            over();
        }
    }
    if panicked {
        return Err(io::Error::other("the copy function panicked"));
    }
    Ok(Kept {
        captured: keeper.finish(),
        first,
    })
}

/// Writes `bytes` to `pipe` and closes it. It stops early, without a
/// failure, when the child closes its end or the waits of `ends` end,
/// because a child that exits without reading its input has answered.
pub(super) fn write<P>(mut pipe: P, bytes: &[u8], ends: &Ends)
where
    P: Write + Pipe,
{
    if sys::unblock(&pipe).is_err() {
        return;
    }
    let mut rest = bytes;
    while !rest.is_empty() {
        match ends.ready(&pipe, true) {
            Ok(true) => {}
            Ok(false) | Err(_) => return,
        }
        match pipe.write(rest) {
            Ok(0) => return,
            Ok(written) => rest = &rest[written..],
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock
                ) => {}
            Err(_) => return,
        }
    }
}
