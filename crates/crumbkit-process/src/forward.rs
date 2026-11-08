//! Stops every live child group when the tool itself receives SIGINT,
//! SIGTERM or SIGHUP, runs the tool's clean-ups, and then takes the
//! signal's default action. It exists on Unix with the `forward` feature.
//!
//! [`install`] starts one thread for the life of the process. When a
//! signal arrives, the thread:
//!
//! 1. takes the registry's lock and holds it until the process ends, so no
//!    new child starts;
//! 2. sends the received signal, then SIGCONT, to every listed group, and
//!    to a listed child that moved to another group;
//! 3. waits up to [`FORWARD_GRACE`] until every listed child has exited;
//! 4. sends SIGKILL to every listed group;
//! 5. runs each clean-up that [`clean_up`] registered;
//! 6. takes the signal's default action, which ends the tool as the signal
//!    would have.
//!
//! A run lists its child while it holds the registry's lock around the
//! start, when the child leads a group of its own
//! ([`Scope::Group`](crate::Scope::Group)). The run unlists it under the
//! same lock before it reaps it, so the forwarder never signals a process
//! id that another process may have taken.
//!
//! The forwarder must be the only handler that ends the tool on SIGINT,
//! SIGTERM or SIGHUP. A tool that keeps crumbkit-usage's log beside the
//! forwarder starts it with `start_under(forward::clean_up)`, so the log
//! starts no signal thread. The line is the first clean-up only when
//! `start_under` runs before anything else registers one. Plain
//! `start` beside [`install`] leaves two handlers that each end the tool,
//! and neither crate can detect that (ADR-0179).
//!
//! A tool that hands the terminal to a child with crumbkit-child calls
//! [`handed_over`] just before the hand-over, so Ctrl-C ends only the child.
//!
//! ```no_run
//! fn remove_lock_file() {}
//!
//! crumbkit_process::forward::clean_up(remove_lock_file);
//! crumbkit_process::forward::install()?;
//! # Ok::<(), std::io::Error>(())
//! ```

use std::io;
use std::process::Child;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};
use std::thread::Builder;
use std::time::{Duration, Instant};

use rustix::process::{Pid, Signal as Raw};
use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
use signal_hook::iterator::Signals;

use crate::sys;

#[cfg(test)]
mod tests;

/// How long the forwarder waits for the listed children to exit after it
/// sends them the tool's signal, before it sends SIGKILL.
pub const FORWARD_GRACE: Duration = Duration::from_secs(1);

/// How long the forwarder sleeps between checks on the listed children.
const CHECK: Duration = Duration::from_millis(10);

/// The live children that lead a group of their own, by process id.
static LISTED: Mutex<Vec<Pid>> = Mutex::new(Vec::new());

/// The functions that the forwarder runs before the tool ends.
static CLEAN_UPS: Mutex<Vec<fn()>> = Mutex::new(Vec::new());

/// Whether a child has taken the terminal, which [`handed_over`] records.
static HANDED_OVER: AtomicBool = AtomicBool::new(false);

/// Whether the forwarder's thread started, which [`install`] decides once.
static INSTALLED: OnceLock<Result<(), io::ErrorKind>> = OnceLock::new();

/// Starts the thread that forwards SIGINT, SIGTERM and SIGHUP to every
/// live child group. A second call does nothing and returns the result of
/// the first.
///
/// The thread stays for the life of the process, because removing a
/// signal-hook handler does not restore the signal's default action. The
/// handler replaces an inherited disposition that ignores SIGINT or SIGHUP,
/// as `nohup` sets, because no safe API reads that disposition.
///
/// # Errors
///
/// Returns the operating system's error kind when the handlers cannot be
/// installed or the thread cannot start.
pub fn install() -> io::Result<()> {
    match INSTALLED.get_or_init(start) {
        Ok(()) => Ok(()),
        Err(kind) => Err(io::Error::new(
            *kind,
            "cannot start the thread that forwards signals",
        )),
    }
}

/// Registers `clean_up` to run when the forwarder has stopped the listed
/// groups and before the tool ends. The clean-ups run in the order that
/// they were registered, each function is kept once, and a clean-up that
/// panics does not stop the others. A clean-up that never returns stops the
/// later ones, so a tool that must write crumbkit-usage's line calls
/// `start_under(forward::clean_up)` before it registers anything else.
///
/// A clean-up must not start a run with
/// [`Scope::Group`](crate::Scope::Group), or wait for a thread that runs
/// one. The forwarder holds the list's lock while the clean-ups run, and
/// such a run waits for that lock forever, so the tool never ends. A run
/// with [`Scope::Child`](crate::Scope::Child) takes no lock.
pub fn clean_up(clean_up: fn()) {
    let mut clean_ups = CLEAN_UPS.lock().unwrap_or_else(PoisonError::into_inner);
    if !clean_ups
        .iter()
        .any(|kept| std::ptr::fn_addr_eq(*kept, clean_up))
    {
        clean_ups.push(clean_up);
    }
}

/// Tells the forwarder that a child is about to take the terminal. From now
/// until the tool ends, the forwarder drops SIGINT, as crumbkit-child ignores
/// it on Unix (ADR-0042, ADR-0179), so Ctrl-C reaches only the child.
/// SIGTERM and SIGHUP still reach the listed groups as before. The child
/// that takes the terminal is not on the forwarder's list, so a SIGTERM or
/// SIGHUP sent to the tool during a hand-over ends the tool and leaves that
/// child on the terminal.
///
/// A tool that hands over the terminal beside the forwarder calls this just
/// before `crumbkit_child::hand_over`.
pub fn handed_over() {
    HANDED_OVER.store(true, Ordering::SeqCst);
}

/// Installs the handlers and starts the forwarder's thread.
fn start() -> Result<(), io::ErrorKind> {
    let mut signals = Signals::new([SIGINT, SIGTERM, SIGHUP]).map_err(|error| error.kind())?;
    Builder::new()
        .name("crumbkit-process forward".to_owned())
        .spawn(move || {
            // RUST-10: `forward` never returns, so a second signal during
            // the grace or the clean-ups is never read, and nothing bounds a
            // clean-up. A `register_conditional_default` on a flag set after
            // the SIGKILL step replaces this if a tool reports a hung
            // clean-up.
            for raw in signals.forever() {
                if raw == SIGINT && HANDED_OVER.load(Ordering::SeqCst) {
                    continue;
                }
                forward(raw);
            }
        })
        .map(drop)
        .map_err(|error| error.kind())
}

/// Stops every listed group on the signal `raw`, runs the clean-ups, and
/// ends the tool as `raw` would have.
fn forward(raw: i32) {
    // The lock stays held until the tool ends, so no child starts after the
    // signal and no listed child is reaped while the forwarder signals it.
    let listed = listed_children();
    if let Some(signal) = Raw::from_named_raw(raw) {
        signal_each(&listed, signal);
    }
    let until = Instant::now().checked_add(FORWARD_GRACE);
    while until.is_some_and(|until| Instant::now() < until)
        && listed
            .iter()
            .any(|pid| !sys::has_exited(*pid).unwrap_or(true))
    {
        std::thread::sleep(CHECK);
    }
    signal_each(&listed, Raw::KILL);
    let clean_ups = CLEAN_UPS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    for clean_up in clean_ups {
        // A clean-up that panics must not keep the tool from ending as the
        // signal says.
        let _ = std::panic::catch_unwind(clean_up);
    }
    // For SIGINT, SIGTERM and SIGHUP this call does not return: it restores
    // the default action and raises the signal, and falls back to `abort`.
    // The lock on the list is therefore never released to a later run.
    let _ = signal_hook::low_level::emulate_default_handler(raw);
}

/// Sends `signal` to each group of `listed`. A refusal is skipped, because
/// the SIGKILL that follows still applies.
fn signal_each(listed: &[Pid], signal: Raw) {
    for pid in listed {
        let _ = sys::signal_group(*pid, signal);
    }
}

/// Starts a child with `spawn` while it holds the registry's lock, and
/// lists the child when it starts.
pub(crate) fn listed(spawn: impl FnOnce() -> io::Result<Child>) -> io::Result<Child> {
    let mut listed = listed_children();
    let child = spawn()?;
    listed.push(Pid::from_child(&child));
    Ok(child)
}

/// Unlists the child `pid` and reaps it with `reap`, both under the
/// registry's lock.
pub(crate) fn unlisted<T>(pid: Pid, reap: impl FnOnce() -> T) -> T {
    let mut listed = listed_children();
    listed.retain(|listed| *listed != pid);
    reap()
}

/// Returns the locked list. Each change is a single push or retain, so a
/// panic while the lock was held cannot leave it half changed.
fn listed_children() -> MutexGuard<'static, Vec<Pid>> {
    LISTED.lock().unwrap_or_else(PoisonError::into_inner)
}
