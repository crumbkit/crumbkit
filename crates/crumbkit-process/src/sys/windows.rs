//! A group on Windows: the child runs in a job object of process-wrap,
//! which starts the child suspended and adds it to the job before it runs,
//! so every process that the child starts is in the job too.
//!
//! Ending the job ends every process in it and closes the pipes they held,
//! so a stop needs no group signal and no wait without reaping, and a reader
//! that blocks on a pipe ends once the job ends. A stop ends the job at once
//! with no graceful step, because a graceful step would send Ctrl-Break
//! through `GenerateConsoleCtrlEvent`, which needs unsafe code (ADR-0171).
//!
//! process-wrap's std job object does not ask Windows to end the job's
//! processes when the job's last handle closes. [`crate::Group`] ends the
//! job when it is dropped, which covers a normal return and a panic that
//! unwinds, but a tool that is killed outright or crashes leaves the job's
//! processes running.

use std::io;
use std::process::{ChildStderr, ChildStdin, ChildStdout, Command, ExitStatus};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use process_wrap::std::{ChildWrapper, CommandWrap, JobObject};

use crate::{Scope, Signal, Status};

/// The handle of a pipe.
pub(crate) use std::os::windows::io::AsHandle as Pipe;

/// A stop on Windows ends the job at once.
pub(crate) const GRACEFUL: bool = false;

/// How long a wait sleeps between checks on whether the child has exited.
///
/// RUST-10: Windows waits by checking the child's state, because the safe
/// API of std and process-wrap has no wait that leaves the child for
/// another thread to signal. It handles every child at the cost of up to
/// this delay after an exit. A wait on the process handle replaces it when
/// a safe API offers one.
const CHECK: Duration = Duration::from_millis(10);

/// A started child, behind a lock that its waiter shares.
#[derive(Debug)]
pub(crate) struct Child {
    wrapped: Arc<Mutex<Box<dyn ChildWrapper>>>,
    scope: Scope,
}

impl Child {
    /// Starts `command`, in a new job object when `scope` asks for a group.
    pub(crate) fn spawn(command: Command, scope: Scope) -> io::Result<Self> {
        let mut wrap = CommandWrap::from(command);
        if scope == Scope::Group {
            wrap.wrap(JobObject);
        }
        let child = wrap.spawn()?;
        Ok(Self {
            wrapped: Arc::new(Mutex::new(child)),
            scope,
        })
    }

    /// Takes the child's standard input, when it is a pipe.
    pub(crate) fn take_stdin(&mut self) -> Option<ChildStdin> {
        locked(&self.wrapped).stdin().take()
    }

    /// Takes the child's standard output, when it is a pipe.
    pub(crate) fn take_stdout(&mut self) -> Option<ChildStdout> {
        locked(&self.wrapped).stdout().take()
    }

    /// Takes the child's standard error, when it is a pipe.
    pub(crate) fn take_stderr(&mut self) -> Option<ChildStderr> {
        locked(&self.wrapped).stderr().take()
    }

    /// Ends the job, or the child alone with [`Scope::Child`], whatever
    /// `signal` is, because Windows has no graceful step.
    pub(crate) fn signal(&self, _signal: Signal) -> io::Result<()> {
        locked(&self.wrapped).start_kill()
    }

    /// Returns true when the child has exited.
    pub(crate) fn has_exited(&self) -> io::Result<bool> {
        Ok(locked(&self.wrapped).try_wait()?.is_some())
    }

    /// Returns a waiter that blocks until the child exits.
    pub(crate) fn waiter(&self) -> Waiter {
        Waiter(Arc::clone(&self.wrapped))
    }

    /// Waits until the child exits, ends the job, which kills what is left
    /// in it, and returns the child's exit status. With [`Scope::Child`] it
    /// ends nothing.
    pub(crate) fn reap(&mut self) -> io::Result<ExitStatus> {
        self.waiter().wait()?;
        let mut child = locked(&self.wrapped);
        if self.scope == Scope::Group {
            child.start_kill()?;
        }
        // process-wrap's `wait` waits for a message from the job, which an
        // earlier `try_wait` may have taken, so the reap checks again
        // instead. The child has exited, so the first check finds it.
        loop {
            if let Some(status) = child.try_wait()? {
                return Ok(status);
            }
            std::thread::sleep(CHECK);
        }
    }
}

/// Blocks until one child exits.
#[derive(Clone, Debug)]
pub(crate) struct Waiter(Arc<Mutex<Box<dyn ChildWrapper>>>);

impl Waiter {
    /// Waits until the child exits.
    pub(crate) fn wait(self) -> io::Result<()> {
        loop {
            if locked(&self.0).try_wait()?.is_some() {
                return Ok(());
            }
            std::thread::sleep(CHECK);
        }
    }
}

/// Returns the locked child. Each holder only calls process-wrap, so a
/// panic while the lock was held leaves nothing half changed.
fn locked(child: &Mutex<Box<dyn ChildWrapper>>) -> MutexGuard<'_, Box<dyn ChildWrapper>> {
    child.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Returns how a child with `status` ended. A Windows status always has a
/// code, and a job that the crate ended gives the code 1.
pub(crate) fn status_of(status: ExitStatus) -> Status {
    Status::Code(status.code().unwrap_or(1))
}

/// Returns true at once, because a Windows pipe cannot be polled.
///
/// RUST-10: a reader on Windows blocks on its pipe until the pipe closes.
/// Ending the job closes every pipe that the job's processes held, so this
/// handles every child in a group. It misses a process that held a pipe
/// and is outside the job, such as a child started with [`Scope::Child`]
/// whose own child outlives it, which holds the run until it exits. A read
/// that waits on the pipe's handle with a timeout replaces it when a safe
/// API offers one, and the run's time bound then holds on Windows too.
#[expect(
    clippy::unnecessary_wraps,
    reason = "the Unix function of the same name can fail, and the run calls both alike"
)]
pub(crate) fn ready(_pipe: impl Pipe, _write: bool, _timeout: Duration) -> io::Result<bool> {
    Ok(true)
}

/// Does nothing, because a write to a Windows pipe ends once the job ends.
///
/// RUST-10: the writer of a child's input blocks on its pipe until the
/// child reads it or the pipe closes. Ending the job closes the pipe for a
/// child in a group. It misses a child started with [`Scope::Child`] whose
/// own child holds the input pipe open without reading it, and the writer
/// then holds the run until that process exits. A write that waits on the
/// pipe's handle with a timeout replaces it when a safe API offers one.
#[expect(
    clippy::unnecessary_wraps,
    reason = "the Unix function of the same name can fail, and the run calls both alike"
)]
pub(crate) fn unblock(_pipe: impl Pipe) -> io::Result<()> {
    Ok(())
}
