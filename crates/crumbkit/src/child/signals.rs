//! The tool's SIGINT and SIGQUIT while a child holds the terminal.
//!
//! Ctrl-C and Ctrl-\ send SIGINT and SIGQUIT to every process in the
//! terminal's foreground process group, which holds the tool and its child.
//! While a child runs, the tool ignores both, as POSIX `system()` does, so
//! only the child acts on them. The tool ignores them with a handler of
//! signal-hook, which does nothing while a child runs and takes the signal's
//! default action otherwise. A handler, unlike an ignored disposition,
//! resets to the default action when the child starts its program, so the
//! child receives both signals as it would without the tool.
//!
//! A usage log started with `Signals::Crumbkit` already listens for SIGINT
//! and lets it end the process after it writes the run's line. Then this
//! module installs no SIGINT handler, and the log's signal thread asks
//! [`holds_terminal`] and leaves the process running while a child holds the
//! terminal (ADR-0029). A usage log started with `Signals::Tool` has already
//! replaced SIGINT's default action, so the tool's own handler sees SIGINT
//! while the child runs.

#[cfg(unix)]
use std::sync::Arc;
#[cfg(unix)]
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

/// Whether a child holds the terminal now.
static HOLDING: AtomicBool = AtomicBool::new(false);

/// The flag that signal-hook's handlers read: true when no child holds the
/// terminal, so a signal takes its default action. It is set once, by the
/// first child, and is `None` when no handler could be installed.
#[cfg(unix)]
static NO_CHILD: OnceLock<Option<Arc<AtomicBool>>> = OnceLock::new();

/// Runs `child`, which starts a child process and waits for it, with SIGINT
/// and SIGQUIT ignored by the tool, and returns what `child` returned.
pub(super) fn while_running<R>(child: impl FnOnce() -> R) -> R {
    #[cfg(unix)]
    let no_child = NO_CHILD.get_or_init(install).as_deref();
    #[cfg(unix)]
    if let Some(no_child) = no_child {
        no_child.store(false, Ordering::SeqCst);
    }
    HOLDING.store(true, Ordering::SeqCst);
    let result = child();
    HOLDING.store(false, Ordering::SeqCst);
    #[cfg(unix)]
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

/// Installs the handlers of SIGQUIT and, unless the usage log listens for
/// it, SIGINT, which take each signal's default action while the returned
/// flag is true. It returns `None` when signal-hook cannot install the
/// handlers, and the tool then keeps both signals' default actions.
#[cfg(unix)]
fn install() -> Option<Arc<AtomicBool>> {
    use signal_hook::consts::{SIGINT, SIGQUIT};
    use signal_hook::flag::register_conditional_default;

    let no_child = Arc::new(AtomicBool::new(true));
    register_conditional_default(SIGQUIT, Arc::clone(&no_child)).ok()?;
    if !crate::usage::listens() {
        register_conditional_default(SIGINT, Arc::clone(&no_child)).ok()?;
    }
    Some(no_child)
}
