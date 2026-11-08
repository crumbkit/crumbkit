//! The signal thread of the usage log, which [`start`](super::start) starts
//! and [`start_under`](super::start_under) does not.

use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::thread;

use signal_hook::consts::SIGINT;

use super::{LISTENS, RECORDER, write};
use crate::observer;
use crate::platform::Interrupts;

/// Starts the thread that listens for SIGINT and SIGTERM, or for SIGINT
/// alone on Windows, and returns once the thread listens.
///
/// The thread writes the line on the first signal. When the main thread is
/// already writing the line, the signal thread waits for it, for at most
/// [`LINE_WAIT`](super::LINE_WAIT), as [`write`] describes. After the line, the thread ends
/// the process as the signal would have without the log. The thread leaves
/// the process running on a SIGINT that arrives once crumbkit reported that
/// a child took the terminal, and on Windows before it reported that the
/// child gave the terminal back, because Ctrl-C then belongs to the child
/// (ADR-0042).
//
// RUST-10: the thread takes both signals from the start of the run. A
// process that started with one of them ignored now ends on it, such as a
// background job of a non-interactive shell, which starts with SIGINT
// ignored. A tool that handles one of them itself ends on it too, so such a
// tool calls `start_under`, which starts no thread (ADR-0179). Reading each signal's disposition
// before taking it replaces this, which needs the `sigaction` call that
// crumbkit's ban on unsafe code rules out today.
pub(super) fn watch_signals() {
    let (listening, listens) = mpsc::channel();
    let spawned = thread::Builder::new()
        .name("crumbkit-usage".to_owned())
        .spawn(move || {
            let interrupts = Interrupts::listen();
            // The flag is set before the main thread goes on, so a child
            // that `main` starts next already sees it.
            if interrupts.is_some() {
                LISTENS.store(true, Ordering::SeqCst);
            }
            let _ = listening.send(());
            let Some(mut interrupts) = interrupts else {
                return;
            };
            while let Some(signal) = interrupts.wait() {
                if signal == SIGINT && observer::held_terminal() {
                    continue;
                }
                // The guard ends the process even when writing the line
                // panics, which would otherwise drop the listener and leave
                // the signal ignored.
                let _ending = Ending {
                    signal,
                    end: Interrupts::end_process,
                };
                write(&RECORDER, true);
                return;
            }
        });
    if spawned.is_ok() {
        // The thread sends once it listens, and a thread that ended early
        // drops the sender, which ends the wait too.
        let _ = listens.recv();
    }
}

/// The guard that ends the process with its signal when it drops, as the
/// signal would have without the log.
pub(super) struct Ending {
    /// The signal that ends the process.
    pub(super) signal: i32,
    /// What ends the process, which is [`Interrupts::end_process`] outside
    /// the tests.
    pub(super) end: fn(i32),
}

impl Drop for Ending {
    fn drop(&mut self) {
        (self.end)(self.signal);
    }
}
