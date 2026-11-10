//! What a run keeps of each output stream.

use std::fmt;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;

use crate::LineQueue;

#[cfg(test)]
mod tests;

/// The bytes that a tail keeps beyond its limit before it drops the oldest,
/// so a long stream is not moved on every read.
const SLACK: usize = 8 * 1024;

/// What a run keeps of one output stream of the child.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub enum Capture {
    /// The stream goes to the null device, and the run keeps nothing.
    #[default]
    Discard,
    /// The stream goes to the tool's own stream, and the run keeps nothing.
    ///
    /// A child in a group of its own is in the background of the terminal.
    /// It can write there unless the terminal has `tostop` set, and it is
    /// stopped when it reads. A stopped child holds a run with no deadline. For a step that
    /// the person answers, use [`Scope::Child`](crate::Scope::Child) with
    /// [`Input::Inherit`](crate::Input::Inherit), so the child stays in the
    /// tool's foreground group and the tool carries on when it ends. Ctrl-C
    /// then reaches both.
    Inherit,
    /// The run keeps the stream's first `limit` bytes. A stream longer than
    /// `limit` bytes is over its limit, and `over` says what the run does
    /// then.
    Whole {
        /// The most bytes that the run keeps.
        limit: usize,
        /// What the run does when the stream passes `limit` bytes.
        over: OverLimit,
    },
    /// The run reads the whole stream and keeps its last `limit` bytes.
    Tail {
        /// The most bytes that the run keeps.
        limit: usize,
    },
    /// The run splits the stream into lines on each `end` byte and pushes
    /// each line, without that byte, to `queue` while the child runs. A
    /// last line with no end is pushed when the stream ends. The run
    /// keeps no bytes in its [`Captured`].
    ///
    /// The first time the stream passes `total` bytes, the stream is over
    /// its limit, and `over` says what the run does then. No line is pushed
    /// after that. A line that passes `line` bytes does the same with
    /// [`LongLine::Over`], and with [`LongLine::Skip`] the run drops that
    /// line alone and goes on.
    ///
    /// Each queue serves one run, which ends it when it returns.
    Lines {
        /// The queue that the run pushes each line to, from
        /// [`line_queue`](crate::line_queue).
        queue: LineQueue,
        /// The most bytes of one line, without its end.
        line: usize,
        /// The most bytes of the whole stream, line ends and skipped lines
        /// included.
        total: usize,
        /// What the run does when the stream passes a limit.
        over: OverLimit,
        /// What a line longer than `line` bytes does.
        long: LongLine,
        /// The byte that ends a line, such as `b'\n'` or `0`.
        end: u8,
    },
}

/// What a line longer than the limit of one line does to a stream
/// delivered as lines.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum LongLine {
    /// The stream is over its limit, and its `over` policy applies.
    #[default]
    Over,
    /// The run drops that line up to its end and delivers the next line.
    /// [`Captured::long_line`] tells that it dropped at least one line, and
    /// [`Captured::over_limit`] stays false unless the total limit passes.
    Skip,
}

/// What a run does when a stream that it keeps whole or as lines passes
/// its limit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum OverLimit {
    /// The run stops the child with its escalation and ends as
    /// [`Stopped(OverLimit)`](crate::StopReason::OverLimit).
    #[default]
    Stop,
    /// The run reads the rest of the stream and drops it, so the child
    /// never waits on a full pipe and runs to its end.
    Sink,
}

impl Capture {
    /// Returns the standard stream that the child gets for this capture.
    pub(super) fn stdio(&self) -> Stdio {
        match self {
            Self::Discard => Stdio::null(),
            Self::Inherit => Stdio::inherit(),
            Self::Whole { .. } | Self::Tail { .. } | Self::Lines { .. } => Stdio::piped(),
        }
    }

    /// Tells the queue of a stream delivered as lines that the run began
    /// to stop its child, so a full queue drops a line instead of waiting.
    pub(super) fn stopping(&self) {
        if let Self::Lines { queue, .. } = self {
            queue.stopping();
        }
    }

    /// Tells the queue of a stream delivered as lines that the child has
    /// been reaped, so a full queue drops a line once `deadline` has
    /// passed.
    pub(super) fn drain_until(&self, deadline: Instant) {
        if let Self::Lines { queue, .. } = self {
            queue.drain_until(deadline);
        }
    }

    /// Ends the queue of a stream delivered as lines, so the caller's
    /// waits end once the queue is empty.
    pub(super) fn end(&self) {
        if let Self::Lines { queue, .. } = self {
            queue.end();
        }
    }
}

/// What a run kept of one output stream.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Captured {
    bytes: Vec<u8>,
    over_limit: bool,
    long_line: bool,
}

impl Captured {
    /// Returns the kept bytes: the first bytes of a stream kept whole, the
    /// last bytes of a tail, and nothing of a discarded stream.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the kept bytes, consuming what was captured.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Returns true when the stream was longer than its limit, so the kept
    /// bytes, or the lines delivered, are not all of it.
    #[must_use]
    pub const fn over_limit(&self) -> bool {
        self.over_limit
    }

    /// Returns true when the run acted on a line of a stream delivered as
    /// lines that passed the limit of one line: that line took the stream
    /// over its limit with [`LongLine::Over`], or the run dropped it with
    /// [`LongLine::Skip`]. A line does not count when the bytes that first
    /// take it over its own limit also take the stream over the total limit,
    /// and only [`Captured::over_limit`] tells.
    #[must_use]
    pub const fn long_line(&self) -> bool {
        self.long_line
    }
}

/// The caller's function that receives each chunk of one output stream.
#[derive(Clone)]
pub(super) struct OutputCopy(Arc<CopyFn>);

/// The caller's function, which the settings hold and a reader thread calls.
type CopyFn = dyn Fn(&[u8]) + Send + Sync;

impl OutputCopy {
    /// Wraps the caller's function.
    pub(super) fn new(copy: impl Fn(&[u8]) + Send + Sync + 'static) -> Self {
        Self(Arc::new(copy))
    }

    /// Gives `chunk` to the caller's function.
    pub(super) fn send(&self, chunk: &[u8]) {
        (self.0)(chunk);
    }
}

impl fmt::Debug for OutputCopy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OutputCopy(..)")
    }
}

/// Keeps the bytes of one stream as its [`Capture`] says.
#[derive(Debug)]
pub(super) struct Keeper {
    capture: Capture,
    kept: Captured,
    /// The bytes of the line that a stream delivered as lines has not yet
    /// ended.
    line: Vec<u8>,
    /// The bytes that a stream delivered as lines has read in all.
    read: usize,
    /// False once the caller has dropped its side of the line queue.
    listening: bool,
    /// True while a stream delivered as lines drops the rest of a long line.
    skipping: bool,
}

impl Keeper {
    /// Returns a keeper that keeps what `capture` asks for.
    pub(super) fn new(capture: &Capture) -> Self {
        Self {
            capture: capture.clone(),
            kept: Captured::default(),
            line: Vec::new(),
            read: 0,
            listening: true,
            skipping: false,
        }
    }

    /// Keeps what it should of `bytes`, which the stream yielded next, and
    /// returns true when these bytes take a stream whose policy stops the
    /// child over its limit for the first time.
    pub(super) fn add(&mut self, bytes: &[u8]) -> bool {
        let kept = &mut self.kept;
        match self.capture {
            Capture::Discard | Capture::Inherit => false,
            Capture::Lines {
                line,
                total,
                over,
                long,
                end,
                ..
            } => self.add_lines(bytes, line, total, long, end) && over == OverLimit::Stop,
            Capture::Whole { limit, over } => {
                let room = limit.saturating_sub(kept.bytes.len());
                kept.bytes
                    .extend_from_slice(&bytes[..room.min(bytes.len())]);
                if bytes.len() <= room || kept.over_limit {
                    return false;
                }
                kept.over_limit = true;
                over == OverLimit::Stop
            }
            Capture::Tail { limit } => {
                kept.bytes.extend_from_slice(bytes);
                if kept.bytes.len() > limit.saturating_add(SLACK) {
                    kept.bytes.drain(..kept.bytes.len() - limit);
                    kept.over_limit = true;
                }
                false
            }
        }
    }

    /// Splits `bytes` into lines and pushes each complete line, and
    /// returns true when these bytes take the stream over `line` bytes for
    /// one line or `total` bytes in all for the first time.
    fn add_lines(
        &mut self,
        bytes: &[u8],
        line: usize,
        total: usize,
        long: LongLine,
        end: u8,
    ) -> bool {
        if self.kept.over_limit {
            return false;
        }
        let mut rest = bytes;
        loop {
            let (piece, ended) = match rest.iter().position(|byte| *byte == end) {
                Some(at) => (&rest[..at], true),
                None => (rest, false),
            };
            self.read = self.read.saturating_add(piece.len() + usize::from(ended));
            if !self.skipping {
                self.line.extend_from_slice(piece);
            }
            let over_line = self.line.len() > line;
            if self.read > total || (over_line && long == LongLine::Over) {
                self.kept.over_limit = true;
                self.kept.long_line |= over_line && self.read <= total;
                self.line = Vec::new();
                return true;
            }
            if over_line {
                self.skipping = true;
                self.kept.long_line = true;
                self.line = Vec::new();
            }
            if !ended {
                return false;
            }
            if self.skipping {
                self.skipping = false;
            } else {
                self.push_line();
            }
            rest = &rest[piece.len() + 1..];
            if rest.is_empty() {
                return false;
            }
        }
    }

    /// Pushes the line kept so far to the queue of a stream delivered as
    /// lines, unless the caller has dropped its side.
    fn push_line(&mut self) {
        let line = std::mem::take(&mut self.line);
        if let Capture::Lines { queue, .. } = &self.capture
            && self.listening
        {
            self.listening = queue.push(line);
        }
    }

    /// Returns what was kept once the stream has ended. A stream delivered
    /// as lines pushes its last line when that line has no end byte.
    pub(super) fn finish(mut self) -> Captured {
        if matches!(self.capture, Capture::Lines { .. })
            && !self.kept.over_limit
            && !self.line.is_empty()
        {
            self.push_line();
        }
        if let Capture::Tail { limit } = self.capture
            && self.kept.bytes.len() > limit
        {
            self.kept.bytes.drain(..self.kept.bytes.len() - limit);
            self.kept.over_limit = true;
        }
        self.kept
    }
}
