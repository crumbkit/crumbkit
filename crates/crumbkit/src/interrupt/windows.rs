//! The owner's Windows calls.

use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use signal_hook::consts::SIGINT;

/// How often the thread reads the flag that the handler sets.
const POLL: Duration = Duration::from_millis(50);

/// The exit code of a process that Ctrl-C ended, `STATUS_CONTROL_C_EXIT`.
#[expect(
    clippy::cast_possible_wrap,
    reason = "Windows reports the status as the same 32 bits read as unsigned"
)]
const CONTROL_C_EXIT: i32 = 0xC000_013A_u32 as i32;

/// The flag that the SIGINT handler sets.
//
// RUST-10: the owner takes only SIGINT, which Ctrl-C raises. Ctrl-Break and
// the console's close events end the process with no clean-up.
pub(super) struct Listener(Arc<AtomicBool>);

impl Listener {
    /// Takes SIGINT from now on.
    pub(super) fn listen() -> io::Result<Self> {
        let raised = Arc::new(AtomicBool::new(false));
        signal_hook::flag::register(SIGINT, Arc::clone(&raised))?;
        Ok(Self(raised))
    }

    /// Waits for the next SIGINT and returns its number.
    pub(super) fn wait(&mut self) -> Option<i32> {
        loop {
            if self.0.swap(false, Ordering::SeqCst) {
                return Some(SIGINT);
            }
            thread::sleep(POLL);
        }
    }
}

/// Ends the process with `STATUS_CONTROL_C_EXIT`.
pub(super) fn end_process(_signal: i32) {
    signal_hook::low_level::exit(CONTROL_C_EXIT);
}
