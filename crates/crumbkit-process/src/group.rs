//! A started child together with every process that it starts, which the
//! crate stops and reaps as one.

use std::io;
use std::process::{ChildStderr, ChildStdin, ChildStdout, Command, ExitStatus};

use crate::sys;

#[cfg(test)]
mod tests;

/// Which processes a run holds and signals.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Scope {
    /// The child and every process that it starts. On Unix the child leads
    /// a new process group, and each signal goes to the group. On Windows
    /// the child runs in a new job object, and a stop ends the job. The job
    /// object replaces the command's creation flags with its own, which
    /// start the child suspended, so flags that the caller set with
    /// `CommandExt::creation_flags` are lost.
    #[default]
    Group,
    /// The child alone. The child stays in the caller's process group, so a
    /// signal that the caller's terminal sends reaches it too, and each
    /// signal from the crate goes to the child only. A process that the
    /// child starts is never stopped by the crate.
    Child,
}

/// A signal that a run sends to the processes in its scope.
///
/// On Windows every signal ends the child at once: with [`Scope::Group`]
/// it ends the job and every process in it, and with [`Scope::Child`] it
/// ends the child alone. Windows has no graceful step that crumbkit can
/// send without unsafe code (ADR-0171).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Signal {
    /// SIGINT, which Ctrl-C sends.
    Interrupt,
    /// SIGTERM, the polite request to end.
    Terminate,
    /// SIGHUP, which a closed terminal sends.
    Hangup,
    /// SIGKILL, which no process can catch or ignore.
    Kill,
}

/// A started child and the processes that it starts, as its [`Scope`]
/// names them.
///
/// On Unix the child stays unreaped until [`Group::reap`], so its process
/// id cannot pass to another process while a signal could still name it.
/// [`Group::reap`] waits for the child without reaping it, kills what is
/// left of the group, reaps the child, and waits until the group is empty
/// for at most [`REAP_GRACE`](crate::REAP_GRACE). A process that left
/// the group, such as one that called `setsid`, is not stopped.
///
/// A group that is dropped before [`Group::reap`] kills the child and every
/// process in its scope and reaps the child, so a normal return and a
/// panic that unwinds leave nothing running. On Windows the drop ends the
/// job. A tool that is killed outright or crashes skips the drop: on Unix
/// the group keeps running, and on Windows the job's processes keep
/// running, because process-wrap's std job object does not ask Windows to
/// end them when the job's last handle closes.
#[derive(Debug)]
pub(crate) struct Group {
    child: sys::Child,
    status: Option<ExitStatus>,
}

impl Group {
    /// Starts `command` in the scope that `scope` names. The caller sets
    /// the command's program, arguments, environment and standard streams.
    ///
    /// # Errors
    ///
    /// Returns the error of [`Command::spawn`] when the program does not
    /// exist or cannot be started, and on Windows when the job object
    /// cannot be created.
    pub(crate) fn start(command: Command, scope: Scope) -> io::Result<Self> {
        Ok(Self {
            child: sys::Child::spawn(command, scope)?,
            status: None,
        })
    }

    /// Returns true when another process in the group still runs. The
    /// child must have exited and stayed unreaped, so it names the group.
    pub(crate) fn others_alive(&self) -> bool {
        self.child.others_alive()
    }

    /// Returns the child's process id.
    #[cfg(all(test, unix))]
    pub(crate) fn id(&self) -> u32 {
        self.child.id()
    }

    /// Takes the child's standard input, when the command made it a pipe
    /// and nobody has taken it yet.
    pub(crate) fn take_stdin(&mut self) -> Option<ChildStdin> {
        self.child.take_stdin()
    }

    /// Takes the child's standard output, when the command made it a pipe
    /// and nobody has taken it yet.
    pub(crate) fn take_stdout(&mut self) -> Option<ChildStdout> {
        self.child.take_stdout()
    }

    /// Takes the child's standard error, when the command made it a pipe
    /// and nobody has taken it yet.
    pub(crate) fn take_stderr(&mut self) -> Option<ChildStderr> {
        self.child.take_stderr()
    }

    /// Sends `signal` to the processes in the group's scope. On Unix with
    /// [`Scope::Group`], a child that moved to another process group gets
    /// the signal too. A signal that finds every process already gone
    /// succeeds.
    ///
    /// # Errors
    ///
    /// Returns an error when the group has been reaped, and the operating
    /// system's error when it refuses the signal.
    pub(crate) fn signal(&self, signal: Signal) -> io::Result<()> {
        if self.status.is_some() {
            return Err(io::Error::other("the child has been reaped"));
        }
        self.child.signal(signal)
    }

    /// Returns true when the child has exited. The child stays unreaped.
    ///
    /// # Errors
    ///
    /// Returns the operating system's error when it cannot tell.
    pub(crate) fn has_exited(&self) -> io::Result<bool> {
        if self.status.is_some() {
            return Ok(true);
        }
        self.child.has_exited()
    }

    /// Blocks until the child exits, and leaves it unreaped. It waits as
    /// long as the child runs.
    ///
    /// # Errors
    ///
    /// Returns the operating system's error when the wait fails.
    #[cfg(all(test, unix))]
    pub(crate) fn wait_exit(&self) -> io::Result<()> {
        if self.status.is_some() {
            return Ok(());
        }
        self.child.waiter().wait()
    }

    /// Returns a waiter that a thread of the crate's own blocks on.
    pub(crate) fn waiter(&self) -> sys::Waiter {
        self.child.waiter()
    }

    /// Waits until the child exits, kills what is left in the group's
    /// scope, reaps the child and returns its exit status. On Unix it then
    /// waits until the group is empty for at most
    /// [`REAP_GRACE`](crate::REAP_GRACE) after the exit. A second call
    /// returns the same status. It waits as long as the child runs, so a
    /// caller that wants the child gone first sends [`Signal::Kill`].
    ///
    /// # Errors
    ///
    /// Returns the operating system's error when the wait, the kill or the
    /// reap fails.
    pub(crate) fn reap(&mut self) -> io::Result<ExitStatus> {
        if let Some(status) = self.status {
            return Ok(status);
        }
        let status = self.child.reap()?;
        self.status = Some(status);
        Ok(status)
    }
}

impl Drop for Group {
    /// Kills every process in the group's scope and reaps the child, unless
    /// [`Group::reap`] already reaped it. On Unix the drop can therefore
    /// block for [`REAP_GRACE`](crate::REAP_GRACE).
    fn drop(&mut self) {
        if self.status.is_none() {
            let _ = self.child.signal(Signal::Kill);
            let _ = self.reap();
        }
    }
}
