//! The tool's SIGINT and SIGQUIT handlers on Unix, and the signal that
//! ended a child.

use std::os::unix::process::ExitStatusExt as _;
use std::process::ExitStatus;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use signal_hook::consts::{SIGINT, SIGQUIT};
use signal_hook::flag::register_conditional_default;

/// Whether the tool takes the default actions of SIGINT and SIGQUIT again
/// once a child exits. It does not, because a signal sent while the child
/// ran can reach one of the tool's threads after the child has exited, and
/// the handler would then end the tool.
pub(super) const RESTORES: bool = false;

/// Installs the handlers of SIGQUIT and, unless the usage log has taken
/// it, SIGINT, which take each signal's default action while the returned
/// flag is true. It returns `None` when signal-hook cannot install the
/// handlers, and the tool then keeps both signals' default actions.
pub(super) fn install() -> Option<Arc<AtomicBool>> {
    let no_child = Arc::new(AtomicBool::new(true));
    register_conditional_default(SIGQUIT, Arc::clone(&no_child)).ok()?;
    if !crate::usage::listens() {
        register_conditional_default(SIGINT, Arc::clone(&no_child)).ok()?;
    }
    Some(no_child)
}

/// Returns the number of the signal that ended a child with `status`, or
/// `None` when the child exited.
pub(in crate::child) fn ended_by(status: ExitStatus) -> Option<i32> {
    status.signal()
}
