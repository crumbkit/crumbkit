//! The owner's Unix calls.

use std::io;

use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
use signal_hook::low_level::emulate_default_handler;

/// The signal-hook iterator over SIGINT and SIGTERM.
pub(super) struct Listener(Signals);

impl Listener {
    /// Takes SIGINT and SIGTERM from now on.
    pub(super) fn listen() -> io::Result<Self> {
        Signals::new([SIGINT, SIGTERM]).map(Self)
    }

    /// Waits for the next signal and returns its number.
    pub(super) fn wait(&mut self) -> Option<i32> {
        self.0.forever().next()
    }
}

/// Lets `signal` take its default action, which ends the process.
pub(super) fn end_process(signal: i32) {
    let _ = emulate_default_handler(signal);
}
