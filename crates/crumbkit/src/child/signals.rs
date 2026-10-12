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
//! while a child runs and takes the signal's default action otherwise. On
//! Unix, a handler, unlike an ignored disposition, resets to the default
//! action when the child starts its program, so the child receives both
//! signals as it would without the tool. A Windows child never inherits
//! the tool's handlers. On Windows the default action ends the tool with
//! the exit code that Ctrl-C gives, `STATUS_CONTROL_C_EXIT`.
//!
//! A usage log has already taken SIGINT with its signal thread. Then this
//! module installs no SIGINT handler. With `Signals::Crumbkit`, the log's
//! thread asks [`holds_terminal`] and leaves the process running while a
//! child holds the terminal (ADR-0029). With `Signals::Tool`, the log's
//! thread never ends the process, so the tool's own handler decides what
//! SIGINT does whether or not a child runs.

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

#[cfg(unix)]
pub(super) use self::unix::ended_by;
#[cfg(unix)]
use self::unix::install;
#[cfg(windows)]
pub(super) use self::windows::ended_by;
#[cfg(windows)]
use self::windows::install;

/// Whether a child holds the terminal now.
static HOLDING: AtomicBool = AtomicBool::new(false);

/// The flag that signal-hook's handlers read: true when no child holds the
/// terminal, so a signal takes its default action. It is set once, by the
/// first child, and is `None` when no handler could be installed.
static NO_CHILD: OnceLock<Option<Arc<AtomicBool>>> = OnceLock::new();

/// Runs `child`, which starts a child process and waits for it, with the
/// tool's interrupt signals ignored, and returns what `child` returned.
pub(super) fn while_running<R>(child: impl FnOnce() -> R) -> R {
    let no_child = NO_CHILD.get_or_init(install).as_deref();
    if let Some(no_child) = no_child {
        no_child.store(false, Ordering::SeqCst);
    }
    HOLDING.store(true, Ordering::SeqCst);
    let result = child();
    HOLDING.store(false, Ordering::SeqCst);
    if let Some(no_child) = no_child {
        no_child.store(true, Ordering::SeqCst);
    }
    result
}

/// Reports whether a child holds the terminal now, which the usage log's
/// signal thread asks before it lets SIGINT end the process.
#[cfg(feature = "usage-log")]
pub(crate) fn holds_terminal() -> bool {
    HOLDING.load(Ordering::SeqCst)
}
