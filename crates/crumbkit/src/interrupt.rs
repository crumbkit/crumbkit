//! Lets a command that works through many items stop before its next item
//! when the person presses Ctrl-C once, and ends the process when the person
//! presses it again. It exists under the `interrupt` feature (ADR-0240).
//!
//! A tool calls [`install`] once in `main`. From then on, one thread owns
//! SIGINT and SIGTERM for the life of the process. A command that works
//! through items calls [`Interrupts::cooperate`] to open a window and
//! [`Interrupt::check`] before each item:
//!
//! ```no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let interrupts = crumbkit::interrupt::install()?;
//! let window = interrupts.cooperate();
//! for item in ["a", "b", "c"] {
//!     window.check()?;
//!     // Work on `item`.
//! #   let _ = item;
//! }
//! # Ok(())
//! # }
//! ```
//!
//! The first Ctrl-C inside a window sets a flag, and the process keeps
//! running. [`Interrupt::check`] then returns [`Interrupted`], whose code is
//! [`codes::RUN_INTERRUPTED`] and whose default exit is 130. A second Ctrl-C
//! inside the window runs the clean-ups that [`clean_up`] registered and
//! ends the process as SIGINT does, which a shell reports as 130; on
//! Windows the process exits with `STATUS_CONTROL_C_EXIT`. A Ctrl-C outside
//! a window, and every SIGTERM, run the clean-ups and end the process. A
//! command that never calls `check` therefore ignores the first Ctrl-C until
//! its window closes, and the window is closed by dropping the [`Interrupt`].
//!
//! While a child holds the terminal (ADR-0042), the owner drops SIGINT.
//! [`events::handles_interrupt`](crate::events::handles_interrupt) answers
//! true once [`install`] ran, so crumbkit-child installs no handler of its
//! own.
//!
//! The owner must be the only handler that ends the process on SIGINT or
//! SIGTERM. A tool that keeps the usage log starts it with
//! `crumbkit_usage::start_under(crumbkit::interrupt::clean_up)`, so the line
//! is written once, before the process ends. A tool that uses
//! crumbkit-process's `forward` feature does not call [`install`], because
//! each of the two ends the tool on SIGINT.

use std::fmt;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock, PoisonError};
use std::thread::Builder;

use signal_hook::consts::SIGINT;

use crate::events::{Observer, Observing, observe};
use crate::{Code, Failure, Recovery, codes};

#[cfg(unix)]
#[path = "interrupt/unix.rs"]
mod platform;
#[cfg(windows)]
#[path = "interrupt/windows.rs"]
mod platform;
#[cfg(test)]
mod tests;

/// Whether a window is open, which [`Interrupts::cooperate`] sets and
/// dropping the [`Interrupt`] clears.
static OPEN: AtomicBool = AtomicBool::new(false);

/// Whether one Ctrl-C arrived inside the open window.
static REQUESTED: AtomicBool = AtomicBool::new(false);

/// Whether a child holds the terminal, which `events::handed_over` records
/// and, on Windows, `events::taken_back` clears.
static HELD: AtomicBool = AtomicBool::new(false);

/// The functions that the owner runs before the process ends.
static CLEAN_UPS: Mutex<Vec<fn()>> = Mutex::new(Vec::new());

/// What the first [`install`] decided.
static INSTALLED: OnceLock<Result<(), InstallError>> = OnceLock::new();

/// Why [`install`] could not take the signals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum InstallError {
    /// Four observers were already installed, so crumbkit could not learn
    /// when a child takes the terminal (see [`events::observe`](crate::events::observe)).
    ObserversFull,
    /// The run answers a shell's completion request, which takes no signal.
    Completing,
    /// The operating system refused a signal handler or the thread.
    Os(io::ErrorKind),
}

impl fmt::Display for InstallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ObserversFull => f.write_str(
                "four observers are installed already, so the interrupt owner has no slot",
            ),
            Self::Completing => f.write_str("the run answers a completion request"),
            Self::Os(kind) => write!(f, "the operating system refused the signal handler: {kind}"),
        }
    }
}

impl std::error::Error for InstallError {}

/// The proof that [`install`] took SIGINT and SIGTERM, which opens windows.
#[derive(Debug)]
pub struct Interrupts(());

/// An open window: a Ctrl-C inside it asks the command to stop instead of
/// ending the process. Dropping it closes the window and forgets the
/// request.
///
/// One window is open at a time. A second [`Interrupts::cooperate`] while a
/// window is open shares the flag, and dropping either closes the window.
#[derive(Debug)]
pub struct Interrupt(());

/// The failure that [`Interrupt::check`] returns after a Ctrl-C.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Interrupted;

impl fmt::Display for Interrupted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the command stopped before its next item because of Ctrl-C")
    }
}

impl std::error::Error for Interrupted {}

impl Failure for Interrupted {
    fn code(&self) -> Code<'_> {
        codes::RUN_INTERRUPTED
    }

    fn recovery(&self) -> Recovery {
        Recovery::Retry
    }
}

/// Takes SIGINT and SIGTERM for the life of the process and returns the
/// token that opens windows. Call it once, first in `main`. A later call
/// returns another token and takes nothing more.
///
/// The owner stays installed until the process ends, because signal-hook
/// does not restore a signal's default action when its last handler goes.
//
// RUST-10: the owner takes both signals from the start. A process that
// started with one ignored, such as a background job of a non-interactive
// shell, now ends on it. Reading each signal's disposition before taking it
// replaces this, which needs the `sigaction` call that crumbkit's ban on
// unsafe code rules out today.
///
/// # Errors
///
/// Returns an [`InstallError`] when no observer slot is free, when the run
/// answers a completion request, or when the operating system refuses. The
/// tool then keeps the signals' default actions.
pub fn install() -> Result<Interrupts, InstallError> {
    (*INSTALLED.get_or_init(start)).map(|()| Interrupts(()))
}

/// Registers `clean_up` to run before the owner ends the process. The
/// clean-ups run in the order they were registered, each function is kept
/// once, and a clean-up that panics does not stop the others.
pub fn clean_up(clean_up: fn()) {
    let mut clean_ups = CLEAN_UPS.lock().unwrap_or_else(PoisonError::into_inner);
    if !clean_ups
        .iter()
        .any(|kept| std::ptr::fn_addr_eq(*kept, clean_up))
    {
        clean_ups.push(clean_up);
    }
}

impl Interrupts {
    /// Opens a window, with the flag clear.
    #[must_use]
    pub fn cooperate(self) -> Interrupt {
        REQUESTED.store(false, Ordering::SeqCst);
        OPEN.store(true, Ordering::SeqCst);
        Interrupt(())
    }
}

impl Interrupt {
    /// Returns [`Interrupted`] once a Ctrl-C arrived in this window.
    ///
    /// # Errors
    ///
    /// Returns [`Interrupted`] after the first Ctrl-C of the window.
    pub fn check(&self) -> Result<(), Interrupted> {
        if self.requested() {
            Err(Interrupted)
        } else {
            Ok(())
        }
    }

    /// Reports whether a Ctrl-C arrived in this window.
    #[must_use]
    pub fn requested(&self) -> bool {
        REQUESTED.load(Ordering::SeqCst)
    }
}

impl Drop for Interrupt {
    fn drop(&mut self) {
        OPEN.store(false, Ordering::SeqCst);
        REQUESTED.store(false, Ordering::SeqCst);
    }
}

/// The observer that tells crumbkit-child that the owner has SIGINT and
/// learns when a child holds the terminal.
struct Owner;

impl Observer for Owner {
    fn handed_over(&self) {
        HELD.store(true, Ordering::SeqCst);
    }

    fn taken_back(&self) {
        HELD.store(false, Ordering::SeqCst);
    }

    fn handles_interrupt(&self) -> bool {
        true
    }
}

/// Installs the observer and starts the owner's thread.
fn start() -> Result<(), InstallError> {
    match observe(&Owner) {
        Observing::Installed | Observing::InstalledLate => {}
        Observing::Full => return Err(InstallError::ObserversFull),
        Observing::Completing => return Err(InstallError::Completing),
    }
    let mut listener =
        platform::Listener::listen().map_err(|error| InstallError::Os(error.kind()))?;
    Builder::new()
        .name("crumbkit-interrupt".to_owned())
        .spawn(move || {
            while let Some(signal) = listener.wait() {
                if decide(signal) == Decision::End {
                    end(signal);
                    return;
                }
            }
        })
        .map(drop)
        .map_err(|error| InstallError::Os(error.kind()))
}

/// What the owner does with a signal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Decision {
    /// The process keeps running.
    Keep,
    /// The clean-ups run and the process ends.
    End,
}

/// Decides what a signal does and records a request, as the lifecycle table
/// says.
fn decide(signal: i32) -> Decision {
    if signal != SIGINT {
        return Decision::End;
    }
    if HELD.load(Ordering::SeqCst) {
        return Decision::Keep;
    }
    if OPEN.load(Ordering::SeqCst) && !REQUESTED.swap(true, Ordering::SeqCst) {
        return Decision::Keep;
    }
    Decision::End
}

/// Runs each clean-up and ends the process as `signal` would have.
fn end(signal: i32) {
    let clean_ups = CLEAN_UPS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    for clean_up in clean_ups {
        // A clean-up that panics must not keep the process from ending.
        let _ = std::panic::catch_unwind(clean_up);
    }
    platform::end_process(signal);
}
