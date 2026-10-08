//! The writers that a tool's result and its error lines go to.

use std::io::{self, Stderr, StdoutLock, Write};

/// The destination of a tool's output: a writer for standard output and a
/// writer for standard error.
///
/// [`finish`](crate::finish) writes to the process's standard streams. Tests
/// build a sink over in-memory writers and pass it to
/// [`finish_to`](crate::finish_to), so they read what a tool would print
/// without running it.
///
/// ```
/// let mut out = Vec::new();
/// let mut err = Vec::new();
/// let sink = crumbkit::Sink::new(&mut out, &mut err);
/// ```
#[derive(Debug)]
pub struct Sink<O, E> {
    /// Receives the result.
    pub(crate) out: O,
    /// Receives error lines.
    pub(crate) err: E,
}

impl<O: Write, E: Write> Sink<O, E> {
    /// Returns a sink that writes the result to `out` and error lines to
    /// `err`.
    #[must_use]
    pub const fn new(out: O, err: E) -> Self {
        Self { out, err }
    }
}

impl Sink<StdoutLock<'static>, Stderr> {
    /// Returns a sink over the process's standard output and standard error.
    /// This is the only place in crumbkit that opens the standard streams
    /// (RUST-4).
    pub(crate) fn standard() -> Self {
        #[expect(
            clippy::disallowed_methods,
            reason = "the sink owns standard output and locks it once for the whole result"
        )]
        let out = io::stdout().lock();
        #[expect(
            clippy::disallowed_methods,
            reason = "the sink owns standard error, where crumbkit reports a failed write or a bad flag"
        )]
        let err = io::stderr();
        Self { out, err }
    }
}
