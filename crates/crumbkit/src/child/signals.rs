//! The tool's interrupt signals while a child holds the terminal.
//!
//! On Unix, Ctrl-C and Ctrl-\ send SIGINT and SIGQUIT to every process in
//! the terminal's foreground process group, which holds the tool and its
//! child. On Windows, Ctrl-C and Ctrl-Break reach every process attached to
//! the console, and the C runtime raises SIGINT and SIGBREAK for them.
//! While a child runs, the tool ignores both of its platform's signals, as
//! POSIX `system()` does, so only the child acts on them.
//!
//! The tool ignores them with a handler of signal-hook, which does nothing
//! while the signals are ignored and takes the signal's default action
//! otherwise. On Unix, a handler, unlike an ignored disposition, resets to
//! the default action when the child starts its program, so the child
//! receives both signals as it would without the tool. A Windows child
//! never inherits the tool's handlers. On Windows the default action ends
//! the tool with the exit code that Ctrl-C gives, `STATUS_CONTROL_C_EXIT`.
//!
//! A handler acts when the signal reaches one of the tool's threads, which
//! can be after the time it was sent. A Ctrl-C sent while the child ran can
//! reach a thread of the tool after the child has exited. On Unix the tool
//! therefore keeps ignoring both signals from the first child until it
//! ends. A command hands the terminal over as its last step, so only the
//! child's result, the observer's last report and the exit remain. On
//! Windows the tool takes the default actions again once the child exits.
//!
//! Before the child starts, this module reports
//! [`Observer::handed_over`](crate::events::Observer::handed_over), and on
//! Windows it reports [`Observer::taken_back`](crate::events::Observer::taken_back)
//! after the child exits. An observer that has already taken SIGINT, as the
//! usage log's signal thread does, says so through
//! [`Observer::handles_interrupt`](crate::events::Observer::handles_interrupt),
//! and this module then installs no SIGINT handler. The usage log's thread
//! leaves the process running on SIGINT once a child took the terminal, and
//! on Windows until the child gave it back (ADR-0042).

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

#[cfg(unix)]
pub(super) use self::unix::ended_by;
#[cfg(unix)]
use self::unix::{RESTORES, install};
#[cfg(windows)]
pub(super) use self::windows::ended_by;
#[cfg(windows)]
use self::windows::{RESTORES, install};

/// The flag that signal-hook's handlers read: true when no child holds the
/// terminal, so a signal takes its default action. It is set once, by the
/// first child, and is `None` when no handler could be installed.
static NO_CHILD: OnceLock<Option<Arc<AtomicBool>>> = OnceLock::new();

/// Runs `child`, which starts a child process and waits for it, with the
/// tool's interrupt signals ignored, and returns what `child` returned. On
/// Unix the signals stay ignored after `child` returns, until the tool ends.
pub(super) fn while_running<R>(child: impl FnOnce() -> R) -> R {
    let no_child = NO_CHILD.get_or_init(install).as_deref();
    if let Some(no_child) = no_child {
        no_child.store(false, Ordering::SeqCst);
    }
    crate::events::handed_over();
    let result = child();
    if RESTORES {
        crate::events::taken_back();
        if let Some(no_child) = no_child {
            no_child.store(true, Ordering::SeqCst);
        }
    }
    result
}
