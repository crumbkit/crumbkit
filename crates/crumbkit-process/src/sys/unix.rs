//! A group on Unix: the child leads a new process group, and every call goes
//! through rustix.
//!
//! The child stays unreaped until [`Child::reap`], so its process id, which
//! is also its group's id, cannot pass to another process while the crate
//! may still signal it. [`Child::reap`] waits for the child without reaping
//! it, kills what is left of its group, reaps it, and waits until the group
//! is empty.

use std::io;
use std::os::fd::AsFd;
use std::os::unix::process::{CommandExt as _, ExitStatusExt as _};
use std::process::{ChildStderr, ChildStdin, ChildStdout, Command, ExitStatus};
use std::time::{Duration, Instant};

use rustix::event::{PollFd, PollFlags, Timespec, poll};
use rustix::io::Errno;
use rustix::process::{
    Pid, Signal as Raw, WaitId, WaitIdOptions, WaitIdStatus, getpgid, kill_process,
    kill_process_group, test_kill_process_group, waitid,
};

#[cfg(feature = "forward")]
use crate::forward::{listed, unlisted};
use crate::{REAP_GRACE, Scope, Signal, Status};

#[cfg(test)]
mod tests;

/// The descriptor of a pipe, which `poll` waits on.
pub(crate) use std::os::fd::AsFd as Pipe;

/// How long a wait sleeps before it checks again on a child that job
/// control stopped or continued, when it found no report to consume.
const STOPPED_CHECK: Duration = Duration::from_millis(10);

/// How long a reap sleeps before it kills the group again, or checks again
/// whether the group is empty.
const EMPTY_CHECK: Duration = Duration::from_millis(1);

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

    /// Returns true when another process in the group still runs. The
    /// caller knows that the child has exited and is unreaped, so its id
    /// still names the group.
    pub(crate) fn others_alive(&self) -> bool {
        self.scope == Scope::Group && others_in_group(self.pid)
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
                .and_then(|()| resume(raw, |cont| kill_process(self.pid, cont)))
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

    /// Waits until the child exits, kills what is left of its group, reaps
    /// the child, and waits until the group is empty. With [`Scope::Child`]
    /// it kills nothing and waits for nothing else, because the group is the
    /// caller's.
    ///
    /// Before the reap it kills the group again for as long as a live
    /// process is left in it, because macOS can miss a process that is being
    /// forked when it signals a group. After the reap it only checks the
    /// group with signal 0, which delivers nothing, until the kernel finds no
    /// process in it, so a killed process that waits for its new parent to
    /// reap it is gone too. Both waits end [`REAP_GRACE`] after the child's
    /// exit, and the reap then returns the child's status with a process of
    /// the group still left.
    ///
    /// A child that leads a group of its own leaves the `forward` feature's
    /// list under the registry's lock before the reap, so the forwarder
    /// never signals a reaped id.
    pub(crate) fn reap(&mut self) -> io::Result<ExitStatus> {
        self.waiter().wait()?;
        if self.scope == Scope::Child {
            return self.process.wait();
        }
        let pid = self.pid;
        let give_up = Instant::now().checked_add(REAP_GRACE);
        // The kill finds no process, or on macOS is refused with `EPERM`,
        // when the exited child is the group's only process, and then there
        // is nothing left to kill.
        until_empty(
            give_up,
            || {
                let _ = kill_process_group(pid, Raw::KILL);
            },
            || missed_by_kill(pid),
        );
        let process = &mut self.process;
        let status = unlisted(pid, || process.wait())?;
        // The group's id cannot pass to another process while any process
        // is in the group, so the check names no other group until it finds
        // the group empty.
        until_empty(
            give_up,
            || {},
            || test_kill_process_group(pid) != Err(Errno::SRCH),
        );
        Ok(status)
    }
}

/// Blocks until one child exits and leaves it unreaped.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Waiter(Pid);

impl Waiter {
    /// Waits until the child exits. The child stays unreaped, so its id
    /// still names it and its group.
    ///
    /// RUST-10: macOS gives a wait for an exit a report that the child
    /// stopped, and keeps giving it while the child stays stopped, because
    /// `WNOWAIT` leaves the report in place. The wait treats any report
    /// other than an exit alike, whether it says stopped or continued: it
    /// consumes the report with a second wait for stops and continues,
    /// which leaves an exit in place, and then waits again. When it finds
    /// no report to consume, it checks again after [`STOPPED_CHECK`], so it
    /// never spins. It wakes once for each stop, and another wait in this
    /// process for the child's stops or continues misses each report that
    /// it consumed. A kqueue `EVFILT_PROC` filter with `NOTE_EXIT`, which
    /// wakes only on the exit, replaces it.
    pub(crate) fn wait(self) -> io::Result<()> {
        loop {
            match waitid(
                WaitId::Pid(self.0),
                WaitIdOptions::EXITED | WaitIdOptions::NOWAIT,
            ) {
                Ok(Some(status)) if ended(&status) => return Ok(()),
                Ok(_) => {
                    if !consume(self.0)? {
                        std::thread::sleep(STOPPED_CHECK);
                    }
                }
                Err(Errno::INTR) => {}
                Err(error) => return Err(error.into()),
            }
        }
    }
}

/// Returns true when a live process is in the group that the exited child
/// `leader` leads.
///
/// Linux counts the unreaped leader as a member of its group, so a signal
/// cannot tell whether anything else is there, and the check reads
/// `/proc`.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn others_in_group(leader: Pid) -> bool {
    let Ok(processes) = std::fs::read_dir("/proc") else {
        return false;
    };
    processes.flatten().any(|process| {
        std::fs::read(process.path().join("stat")).is_ok_and(|stat| {
            live_in_group(
                &String::from_utf8_lossy(&stat),
                leader.as_raw_nonzero().get(),
            )
        })
    })
}

/// Returns true when the exited child's group has a live process. macOS
/// skips a process that has exited when it signals a group, and refuses the
/// signal when no other process is left.
#[cfg(target_os = "macos")]
fn others_in_group(leader: Pid) -> bool {
    test_kill_process_group(leader).is_ok()
}

/// Returns false.
///
/// RUST-10: only Linux and macOS have a checked way to tell a live member
/// of the group from the exited leader, so another Unix reports no
/// survivors. A check for that system replaces this when a tool needs it.
#[cfg(not(any(target_os = "linux", target_os = "android", target_os = "macos")))]
const fn others_in_group(_leader: Pid) -> bool {
    false
}

/// Returns true when a live process is in the group that the exited child
/// `leader` leads after a kill of the group, which on macOS can miss a
/// process that is being forked.
#[cfg(target_os = "macos")]
fn missed_by_kill(leader: Pid) -> bool {
    others_in_group(leader)
}

/// Returns false. Linux delivers a kill of a group to a process that is
/// being forked, and a check of `/proc` would read every process on each
/// reap. Another Unix system gets one kill, because no check for it is
/// tested.
#[cfg(not(target_os = "macos"))]
const fn missed_by_kill(_leader: Pid) -> bool {
    false
}

/// Calls `step` and then `left` until `left` returns false or the time
/// `give_up` passes, and sleeps for [`EMPTY_CHECK`] between the passes. A
/// `give_up` of `None` never passes.
fn until_empty(give_up: Option<Instant>, mut step: impl FnMut(), mut left: impl FnMut() -> bool) {
    loop {
        step();
        if !left() || give_up.is_some_and(|give_up| Instant::now() >= give_up) {
            return;
        }
        std::thread::sleep(EMPTY_CHECK);
    }
}

/// Returns true when `stat`, the text of a `/proc/<id>/stat` file, is a
/// process in `group` that has not exited. The name in that file can hold
/// spaces and parentheses, so the fields are read after its last `)`.
#[cfg(any(target_os = "linux", target_os = "android", test))]
fn live_in_group(stat: &str, group: i32) -> bool {
    let Some((_, fields)) = stat.rsplit_once(')') else {
        return false;
    };
    let mut fields = fields.split_ascii_whitespace();
    let state = fields.next();
    let in_group = fields.nth(1).and_then(|value| value.parse().ok()) == Some(group);
    in_group && !matches!(state, Some("Z" | "X" | "x"))
}

/// Consumes a report that the child `pid` stopped or continued, so that the
/// next wait for its exit blocks, and returns whether it found one.
fn consume(pid: Pid) -> io::Result<bool> {
    let options = WaitIdOptions::STOPPED | WaitIdOptions::CONTINUED | WaitIdOptions::NOHANG;
    loop {
        match waitid(WaitId::Pid(pid), options) {
            Ok(found) => return Ok(found.is_some()),
            Err(Errno::INTR) => {}
            Err(error) => return Err(error.into()),
        }
    }
}

/// Sends `raw` to the group that `pid` leads, and to `pid` itself when it
/// has moved to another group, so the child gets the signal once.
///
/// The kernel refuses a signal with `ESRCH` only when no process that it
/// names is left, and that is no failure, because the caller then waits
/// for an exit that has already happened. A refusal of the group's signal
/// still lets a child that moved get its own, and the first refusal is
/// returned.
///
/// A signal other than SIGKILL is followed by SIGCONT to the same target,
/// because a process that job control stopped, as SIGTTIN stops a child
/// that reads the terminal, handles no other signal until it continues.
pub(crate) fn signal_group(pid: Pid, raw: Raw) -> io::Result<()> {
    let group = sent(kill_process_group(pid, raw))
        .and_then(|()| resume(raw, |cont| kill_process_group(pid, cont)));
    if getpgid(Some(pid)).ok() == Some(pid) {
        return group;
    }
    let moved =
        sent(kill_process(pid, raw)).and_then(|()| resume(raw, |cont| kill_process(pid, cont)));
    group.and(moved)
}

/// Sends SIGCONT with `send` after the signal `raw`, unless `raw` is
/// SIGKILL, which ends a stopped process too.
fn resume(raw: Raw, send: impl FnOnce(Raw) -> rustix::io::Result<()>) -> io::Result<()> {
    if raw == Raw::KILL {
        return Ok(());
    }
    sent(send(Raw::CONT))
}

/// Returns true when `status` reports an exit, and false when it reports
/// a stop or a continue, which macOS also gives to a wait for an exit.
fn ended(status: &WaitIdStatus) -> bool {
    status.exited() || status.killed() || status.dumped()
}

/// Returns true when the child `pid` has exited, and leaves it unreaped.
pub(crate) fn has_exited(pid: Pid) -> io::Result<bool> {
    let options = WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT;
    loop {
        match waitid(WaitId::Pid(pid), options) {
            Ok(status) => return Ok(status.is_some_and(|status| ended(&status))),
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
pub(crate) fn status_of(status: ExitStatus) -> Status {
    match status.code() {
        Some(code) => Status::Code(code),
        None => Status::Signal(status.signal().unwrap_or(0)),
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
