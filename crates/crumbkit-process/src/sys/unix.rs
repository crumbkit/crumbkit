//! A group on Unix: the child leads a new process group, and every call goes
//! through rustix.
//!
//! The child stays unreaped until [`Child::reap`], so its process id, which
//! is also its group's id, cannot pass to another process while the crate
//! may still signal it. [`Child::reap`] waits for the child without reaping
//! it, kills what is left of its group, and only then reaps it.

use std::io;
use std::os::fd::AsFd;
use std::os::unix::process::{CommandExt as _, ExitStatusExt as _};
use std::process::{ChildStderr, ChildStdin, ChildStdout, Command, ExitStatus};
use std::time::Duration;

use rustix::event::{PollFd, PollFlags, Timespec, poll};
use rustix::io::Errno;
use rustix::process::{
    Pid, Signal as Raw, WaitId, WaitIdOptions, getpgid, kill_process, kill_process_group, waitid,
};

#[cfg(feature = "forward")]
use crate::forward::{listed, unlisted};
use crate::{Exit, Scope, Signal};

/// The descriptor of a pipe, which `poll` waits on.
pub(crate) use std::os::fd::AsFd as Pipe;

/// A stop on Unix sends each signal of its escalation and waits its grace
/// before the next one.
pub(crate) const GRACEFUL: bool = true;

/// A started child, the id that names it and its group, and whether it
/// leads that group.
#[derive(Debug)]
pub(crate) struct Child {
    process: std::process::Child,
    pid: Pid,
    scope: Scope,
}

impl Child {
    /// Starts `command`, leading a new process group when `scope` asks for
    /// one.
    /// A child that leads a group of its own is listed for the `forward`
    /// feature while the registry's lock is held around the start.
    pub(crate) fn spawn(mut command: Command, scope: Scope) -> io::Result<Self> {
        let child = if scope == Scope::Group {
            command.process_group(0);
            listed(|| command.spawn())?
        } else {
            command.spawn()?
        };
        let pid = Pid::from_child(&child);
        Ok(Self {
            process: child,
            pid,
            scope,
        })
    }

    /// Returns the child's process id.
    #[cfg(test)]
    pub(crate) fn id(&self) -> u32 {
        self.process.id()
    }

    /// Takes the child's standard input, when it is a pipe.
    pub(crate) fn take_stdin(&mut self) -> Option<ChildStdin> {
        self.process.stdin.take()
    }

    /// Takes the child's standard output, when it is a pipe.
    pub(crate) fn take_stdout(&mut self) -> Option<ChildStdout> {
        self.process.stdout.take()
    }

    /// Takes the child's standard error, when it is a pipe.
    pub(crate) fn take_stderr(&mut self) -> Option<ChildStderr> {
        self.process.stderr.take()
    }

    /// Sends `signal` to the child's group, and to the child itself when it
    /// has moved to another group. With [`Scope::Child`] it sends `signal`
    /// to the child alone.
    ///
    /// macOS refuses a signal to a group whose only process has exited with
    /// `EPERM`, so a refusal after the child exited counts as sent.
    pub(crate) fn signal(&self, signal: Signal) -> io::Result<()> {
        let raw = raw(signal);
        let result = if self.scope == Scope::Child {
            sent(kill_process(self.pid, raw))
        } else {
            signal_group(self.pid, raw)
        };
        match result {
            Err(_) if self.has_exited().unwrap_or(false) => Ok(()),
            result => result,
        }
    }

    /// Returns true when the child has exited, and leaves it unreaped.
    pub(crate) fn has_exited(&self) -> io::Result<bool> {
        has_exited(self.pid)
    }

    /// Returns a waiter that blocks until the child exits.
    pub(crate) const fn waiter(&self) -> Waiter {
        Waiter(self.pid)
    }

    /// Waits until the child exits, kills what is left of its group, and
    /// reaps the child. With [`Scope::Child`] it kills nothing, because the
    /// group is the caller's.
    ///
    /// A child that leads a group of its own leaves the `forward` feature's
    /// list under the registry's lock before the reap, so the forwarder
    /// never signals a reaped id.
    pub(crate) fn reap(&mut self) -> io::Result<ExitStatus> {
        self.waiter().wait()?;
        if self.scope == Scope::Child {
            return self.process.wait();
        }
        // The kill finds no process, or on macOS is refused with `EPERM`,
        // when the exited child is the group's only process, and then there
        // is nothing left to kill.
        let _ = kill_process_group(self.pid, Raw::KILL);
        let process = &mut self.process;
        unlisted(self.pid, || process.wait())
    }
}

/// Blocks until one child exits and leaves it unreaped.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Waiter(Pid);

impl Waiter {
    /// Waits until the child exits. The child stays unreaped, so its id
    /// still names it and its group.
    pub(crate) fn wait(self) -> io::Result<()> {
        loop {
            match waitid(
                WaitId::Pid(self.0),
                WaitIdOptions::EXITED | WaitIdOptions::NOWAIT,
            ) {
                Ok(_) => return Ok(()),
                Err(Errno::INTR) => {}
                Err(error) => return Err(error.into()),
            }
        }
    }
}

/// Sends `raw` to the group that `pid` leads, and to `pid` itself when it
/// has moved to another group, so the child gets the signal once.
///
/// The kernel refuses a signal with `ESRCH` only when no process that it
/// names is left, and that is no failure, because the caller then waits
/// for an exit that has already happened.
pub(crate) fn signal_group(pid: Pid, raw: Raw) -> io::Result<()> {
    sent(kill_process_group(pid, raw))?;
    if getpgid(Some(pid)).ok() == Some(pid) {
        return Ok(());
    }
    sent(kill_process(pid, raw))
}

/// Returns true when the child `pid` has exited, and leaves it unreaped.
pub(crate) fn has_exited(pid: Pid) -> io::Result<bool> {
    let options = WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT;
    loop {
        match waitid(WaitId::Pid(pid), options) {
            Ok(status) => return Ok(status.is_some()),
            Err(Errno::INTR) => {}
            Err(error) => return Err(error.into()),
        }
    }
}

/// Starts a child with `spawn` and lists it for the forwarder of the
/// `forward` feature, which is off, so it only starts the child.
#[cfg(not(feature = "forward"))]
fn listed(
    spawn: impl FnOnce() -> io::Result<std::process::Child>,
) -> io::Result<std::process::Child> {
    spawn()
}

/// Reaps a child with `reap` after it leaves the forwarder's list of the
/// `forward` feature, which is off, so it only reaps the child.
#[cfg(not(feature = "forward"))]
fn unlisted<T>(_pid: Pid, reap: impl FnOnce() -> T) -> T {
    reap()
}

/// Returns the rustix signal of `signal`.
const fn raw(signal: Signal) -> Raw {
    match signal {
        Signal::Interrupt => Raw::INT,
        Signal::Terminate => Raw::TERM,
        Signal::Hangup => Raw::HUP,
        Signal::Kill => Raw::KILL,
    }
}

/// Returns the result of a signal, where `ESRCH`, which says that no process
/// was left to receive it, counts as sent.
fn sent(result: rustix::io::Result<()>) -> io::Result<()> {
    match result {
        Ok(()) | Err(Errno::SRCH) => Ok(()),
        Err(error) => Err(error.into()),
    }
}

/// Returns how a child with `status` ended. A status from a wait has either
/// a code or a signal on Unix, so the signal 0 never appears.
pub(crate) fn exit_of(status: ExitStatus) -> Exit {
    match status.code() {
        Some(code) => Exit::Code(code),
        None => Exit::Signal(status.signal().unwrap_or(0)),
    }
}

/// Waits at most `timeout` until `pipe` can be read, or written when `write`
/// is true, and returns whether it can. A pipe whose other end has closed
/// counts as ready, so the read or write that follows sees the end.
pub(crate) fn ready(pipe: impl AsFd, write: bool, timeout: Duration) -> io::Result<bool> {
    let flags = if write { PollFlags::OUT } else { PollFlags::IN };
    let timeout = Timespec::try_from(timeout).map_err(io::Error::other)?;
    let mut fds = [PollFd::new(&pipe, flags)];
    match poll(&mut fds, Some(&timeout)) {
        Ok(ready) => Ok(ready > 0),
        Err(Errno::INTR) => Ok(false),
        Err(error) => Err(error.into()),
    }
}

/// Lets a write to `pipe` return as soon as it has written what fits, so a
/// write never waits for a child that stopped reading.
pub(crate) fn unblock(pipe: impl AsFd) -> io::Result<()> {
    rustix::io::ioctl_fionbio(pipe, true).map_err(io::Error::from)
}
