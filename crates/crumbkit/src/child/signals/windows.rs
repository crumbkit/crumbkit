//! The tool's SIGINT and SIGBREAK handlers on Windows, and the interrupt
//! that ended a child.

use std::process::ExitStatus;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use signal_hook::consts::{SIGBREAK, SIGINT};
use signal_hook::flag::register_conditional_shutdown;

/// The exit code of a process that Ctrl-C or Ctrl-Break ended,
/// `STATUS_CONTROL_C_EXIT`, as the `i32` that an exit takes.
#[expect(
    clippy::cast_possible_wrap,
    reason = "Windows exit codes are 32-bit, and this one is above `i32::MAX` by design"
)]
const CONTROL_C_EXIT: i32 = 0xC000_013A_u32 as i32;

/// Whether the tool ends on Ctrl-C and Ctrl-Break again once a child
/// exits. It does.
//
// RUST-10: a Ctrl-C sent while the child ran may reach the tool's handler
// after the child has exited and end the tool, as it could on Unix. Nobody
// has tested this on Windows. Keeping the signals ignored until the tool
// ends, as Unix does, replaces this if it happens.
pub(super) const RESTORES: bool = true;

/// Installs the handlers of SIGBREAK and, unless the observer has taken
/// it, SIGINT, which end the process with `STATUS_CONTROL_C_EXIT` while the
/// returned flag is true, as Ctrl-C and Ctrl-Break do without a handler.
/// It returns `None` when signal-hook cannot install the handlers, and the
/// tool then keeps both signals' default actions.
//
// RUST-10: closing the console, logging off and shutting down still end
// the tool while a child runs, because they raise no signal that the C
// runtime passes on. A console control handler replaces this if a tool
// needs to outlive its child there, which needs a call that crumbkit's ban
// on unsafe code rules out today.
pub(super) fn install() -> Option<Arc<AtomicBool>> {
    let no_child = Arc::new(AtomicBool::new(true));
    register_conditional_shutdown(SIGBREAK, CONTROL_C_EXIT, Arc::clone(&no_child)).ok()?;
    if !crate::events::handles_interrupt() {
        register_conditional_shutdown(SIGINT, CONTROL_C_EXIT, Arc::clone(&no_child)).ok()?;
    }
    Some(no_child)
}

/// Returns SIGINT's number, 2, when a child with `status` ended with
/// `STATUS_CONTROL_C_EXIT`, the exit code that Ctrl-C and Ctrl-Break give,
/// and `None` otherwise.
pub(in crate::child) fn ended_by(status: ExitStatus) -> Option<i32> {
    (status.code() == Some(CONTROL_C_EXIT)).then_some(SIGINT)
}
