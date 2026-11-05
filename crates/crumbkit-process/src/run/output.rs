//! What a run keeps of each output stream.

use std::process::Stdio;

#[cfg(test)]
mod tests;

/// The bytes that a tail keeps beyond its limit before it drops the oldest,
/// so a long stream is not moved on every read.
const SLACK: usize = 8 * 1024;

/// What a run keeps of one output stream of the child.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Capture {
    /// The stream goes to the null device, and the run keeps nothing.
    #[default]
    Discard,
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
}

/// What a run does when a stream that it keeps whole passes its limit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OverLimit {
    /// The run stops the child with its escalation and ends as
    /// [`Stopped(OverLimit)`](crate::Stop::OverLimit).
    #[default]
    Stop,
    /// The run reads the rest of the stream and drops it, so the child
    /// never waits on a full pipe and runs to its end.
    Sink,
}

impl Capture {
    /// Returns the standard stream that the child gets for this capture.
    pub(super) fn stdio(self) -> Stdio {
        match self {
            Self::Discard => Stdio::null(),
            Self::Whole { .. } | Self::Tail { .. } => Stdio::piped(),
        }
    }
}

/// What a run kept of one output stream.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Captured {
    bytes: Vec<u8>,
    over_limit: bool,
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
    /// bytes are not all of it.
    #[must_use]
    pub const fn over_limit(&self) -> bool {
        self.over_limit
    }
}

/// Keeps the bytes of one stream as its [`Capture`] says.
#[derive(Debug)]
pub(super) struct Keeper {
    capture: Capture,
    kept: Captured,
}

impl Keeper {
    /// Returns a keeper that keeps what `capture` asks for.
    pub(super) fn new(capture: Capture) -> Self {
        Self {
            capture,
            kept: Captured::default(),
        }
    }

    /// Keeps what it should of `bytes`, which the stream yielded next, and
    /// returns true when these bytes take a stream whose policy stops the
    /// child over its limit for the first time.
    pub(super) fn add(&mut self, bytes: &[u8]) -> bool {
        let kept = &mut self.kept;
        match self.capture {
            Capture::Discard => false,
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

    /// Returns what was kept once the stream has ended.
    pub(super) fn finish(mut self) -> Captured {
        if let Capture::Tail { limit } = self.capture
            && self.kept.bytes.len() > limit
        {
            self.kept.bytes.drain(..self.kept.bytes.len() - limit);
            self.kept.over_limit = true;
        }
        self.kept
    }
}
