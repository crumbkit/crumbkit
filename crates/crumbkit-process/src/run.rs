//! The one-shot run: start a child, write its input, read its output within
//! limits and a deadline, stop it when needed, and reap it.

use std::fmt;
use std::io;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::thread::{Builder, Scope as Threads, ScopedJoinHandle};
use std::time::{Duration, Instant};

use crate::notice::Notice;
use crate::{Cancel, Escalation, Exit, Group, Scope, Signal, Stop, sys};

mod output;
mod pipes;

#[cfg(test)]
mod tests;

use self::output::Keeper;
pub use self::output::{Capture, Captured, OverLimit};
use self::pipes::{Ends, Kept};

/// How long a run keeps reading the child's pipes after it reaps the
/// child. A descendant outside the child's scope that holds a pipe open
/// holds the run no longer than this.
pub const DRAIN_GRACE: Duration = Duration::from_secs(1);

/// What the child reads on its standard input.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Input {
    /// The null device, so the child reads the end of its input at once.
    #[default]
    Null,
    /// These bytes, which a thread of the run writes and then closes. The
    /// thread stops early when the child closes its input.
    Bytes(Vec<u8>),
    /// The tool's own standard input. On Unix a child in a group of its own
    /// that reads a terminal is stopped by SIGTTIN, so a terminal suits
    /// only [`Scope::Child`].
    Inherit,
}

/// The settings of one [`run`]: the child's input, what it keeps of each
/// output stream, when and how it stops the child, and the child's scope.
///
/// The default reads nothing, keeps nothing, has no deadline and no cancel,
/// kills at once when it stops the child, and starts the child in a group
/// of its own.
#[derive(Clone, Debug, Default)]
pub struct Run {
    stdin: Input,
    stdout: Capture,
    stderr: Capture,
    deadline: Option<Instant>,
    cancel: Option<Cancel>,
    escalation: Escalation,
    scope: Scope,
}

impl Run {
    /// Returns the default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets what the child reads on its standard input.
    #[must_use]
    pub fn stdin(mut self, input: Input) -> Self {
        self.stdin = input;
        self
    }

    /// Sets what the run keeps of the child's standard output.
    #[must_use]
    pub fn stdout(mut self, capture: Capture) -> Self {
        self.stdout = capture;
        self
    }

    /// Sets what the run keeps of the child's standard error.
    #[must_use]
    pub fn stderr(mut self, capture: Capture) -> Self {
        self.stderr = capture;
        self
    }

    /// Sets the time at which the run stops a child that has not exited,
    /// which then ends as [`Exit::TimedOut`].
    #[must_use]
    pub const fn deadline(mut self, deadline: Instant) -> Self {
        self.deadline = Some(deadline);
        self
    }

    /// Makes the run watch `cancel`, so [`Cancel::cancel`] stops it.
    #[must_use]
    pub fn cancel(mut self, cancel: &Cancel) -> Self {
        self.cancel = Some(cancel.clone());
        self
    }

    /// Sets the steps that stop the child.
    #[must_use]
    pub fn escalation(mut self, escalation: Escalation) -> Self {
        self.escalation = escalation;
        self
    }

    /// Sets which processes the run holds and signals.
    #[must_use]
    pub const fn scope(mut self, scope: Scope) -> Self {
        self.scope = scope;
        self
    }

    /// Tells the queue of each stream delivered as lines that the run began
    /// to stop its child.
    fn stopping(&self) {
        self.stdout.stopping();
        self.stderr.stopping();
    }
}

/// How a run's child ended and what the run kept of its output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finished {
    exit: Exit,
    stdout: Captured,
    stderr: Captured,
    first_stdout: Option<Duration>,
}

impl Finished {
    /// Returns how the child ended.
    #[must_use]
    pub const fn exit(&self) -> Exit {
        self.exit
    }

    /// Returns what the run kept of standard output.
    #[must_use]
    pub const fn stdout(&self) -> &Captured {
        &self.stdout
    }

    /// Returns what the run kept of standard error.
    #[must_use]
    pub const fn stderr(&self) -> &Captured {
        &self.stderr
    }

    /// Returns what the run kept of standard output and standard error.
    #[must_use]
    pub fn into_output(self) -> (Captured, Captured) {
        (self.stdout, self.stderr)
    }

    /// Returns the time from the child's start to the first byte of its
    /// standard output, or `None` when it printed none or the run discarded
    /// standard output.
    #[must_use]
    pub const fn first_stdout(&self) -> Option<Duration> {
        self.first_stdout
    }
}

/// A run that could not start its child or follow it to its end.
#[derive(Debug)]
#[non_exhaustive]
pub enum RunError {
    /// The program does not exist or cannot be started.
    Start(io::Error),
    /// Waiting for the child, reaping it, reading its output or starting a
    /// thread failed after it started. The child was stopped and reaped.
    Run(io::Error),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Start(_) => "cannot start the program",
            Self::Run(_) => "cannot follow the program to its end",
        })
    }
}

impl std::error::Error for RunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Start(error) | Self::Run(error) => Some(error),
        }
    }
}

/// Runs `command` with the settings of `run` and returns how it ended and
/// what it printed.
///
/// The run sets the command's standard streams, starts it in the scope of
/// [`Run::scope`], and waits until the child exits, the deadline passes,
/// the cancel is cancelled, or a stream whose policy stops the child passes
/// its limit. In the last three cases it stops the child with the
/// [`Escalation`]. Once the child has exited, the run kills what is left of
/// its group, reaps it, and reads its pipes for at most [`DRAIN_GRACE`]
/// more. A run whose deadline passes after the child exited gives the
/// child's own exit.
///
/// On Unix, for a child that the tool may signal, a run takes at most the
/// time to its deadline, plus the graces of its escalation, plus
/// [`DRAIN_GRACE`], as long as the caller reads the lines of each stream
/// delivered as lines. Every thread that it starts is joined before it
/// returns. On Windows a stop ends the job at once, and a reader waits for
/// its pipe to close, which ending the job causes for every process in the
/// job, so a process outside the job that holds a pipe holds the run.
///
/// A stream delivered as lines waits for the caller while its queue is
/// full, and the child then waits on its full pipe. Once the run begins to
/// stop the child, a full queue drops each line instead. The run ends each
/// line queue when it returns, on every path.
///
/// # Errors
///
/// Returns [`RunError::Start`] when the program cannot be started, and
/// [`RunError::Run`] when waiting for, reaping or reading the child fails.
/// The child is stopped and reaped on every path.
pub fn run(mut command: Command, run: &Run) -> Result<Finished, RunError> {
    let _end = EndLines(run);
    if run.cancel.as_ref().is_some_and(Cancel::is_cancelled) {
        return Ok(Finished {
            exit: Exit::Stopped(Stop::Cancelled),
            stdout: Captured::default(),
            stderr: Captured::default(),
            first_stdout: None,
        });
    }
    let input = match &run.stdin {
        Input::Null => Stdio::null(),
        Input::Inherit => Stdio::inherit(),
        Input::Bytes(_) => Stdio::piped(),
    };
    command
        .stdin(input)
        .stdout(run.stdout.stdio())
        .stderr(run.stderr.stdio());
    let started = Instant::now();
    let mut group = Group::start(command, run.scope).map_err(RunError::Start)?;
    let notice = Arc::new(Notice::default());
    if let Some(cancel) = &run.cancel {
        cancel.watch(&notice);
    }
    let ends = Ends::default();
    let followed = std::thread::scope(|threads| {
        let unwind = Unwind {
            group: &mut group,
            ends: &ends,
            run,
        };
        follow(threads, &mut *unwind.group, &notice, &ends, run)
    });
    let (exit, stdout, stderr) = followed.map_err(RunError::Run)?;
    Ok(Finished {
        exit,
        first_stdout: stdout.first.map(|first| first - started),
        stdout: stdout.captured,
        stderr: stderr.captured,
    })
}

/// Ends the line queues of a run when the run returns, on every path.
struct EndLines<'run>(&'run Run);

impl Drop for EndLines<'_> {
    fn drop(&mut self) {
        self.0.stdout.end();
        self.0.stderr.end();
    }
}

/// Ends the threads of a run when [`follow`] returns, which matters when it
/// panics: it kills the child unless the group has been reaped, makes the
/// readers and the writer stop waiting on their pipes, and makes each line
/// queue drop instead of waiting. The scope then joins every thread, and
/// the group's drop reaps the child. On a normal return each of these has
/// already happened or does nothing.
struct Unwind<'a> {
    group: &'a mut Group,
    ends: &'a Ends,
    run: &'a Run,
}

impl Drop for Unwind<'_> {
    fn drop(&mut self) {
        // A reaped group refuses the signal, so the kill never reaches a
        // process id that another process may have taken.
        let _ = self.group.signal(Signal::Kill);
        self.ends.set(Instant::now());
        self.run.stopping();
    }
}

/// The threads of one run: the waiter, the readers and the writer.
struct Started<'scope> {
    waiter: ScopedJoinHandle<'scope, io::Result<()>>,
    stdout: Option<ScopedJoinHandle<'scope, io::Result<Kept>>>,
    stderr: Option<ScopedJoinHandle<'scope, io::Result<Kept>>>,
    stdin: Option<ScopedJoinHandle<'scope, ()>>,
}

/// Follows the child of `group` to its end within `threads`, and returns
/// its exit and what the readers kept.
fn follow<'scope>(
    threads: &'scope Threads<'scope, '_>,
    group: &mut Group,
    notice: &'scope Notice,
    ends: &'scope Ends,
    run: &'scope Run,
) -> io::Result<(Exit, Kept, Kept)> {
    let pipes = (group.take_stdin(), group.take_stdout(), group.take_stderr());
    let exit = group.waiter();
    let waiter = match Builder::new().spawn_scoped(threads, move || {
        let waited = exit.wait();
        notice.set(|flags| flags.exited = true);
        waited
    }) {
        Ok(waiter) => waiter,
        Err(error) => {
            let _ = group.signal(Signal::Kill);
            let _ = group.reap();
            return Err(error);
        }
    };
    let mut started = Started {
        waiter,
        stdout: None,
        stderr: None,
        stdin: None,
    };
    let mut failure = start_pipes(threads, &mut started, pipes, notice, ends, run).err();
    let stopped = if failure.is_some() {
        stop(group, notice, &Escalation::default(), run);
        None
    } else {
        control(group, notice, run)
    };
    let status = group.reap();
    ends.set(
        Instant::now()
            .checked_add(DRAIN_GRACE)
            .unwrap_or_else(Instant::now),
    );
    let exited = joined(started.waiter.join());
    let stdout = started.stdout.map(|reader| joined(reader.join()));
    let stderr = started.stderr.map(|reader| joined(reader.join()));
    if let Some(writer) = started.stdin {
        let _ = writer.join();
    }
    if let Some(failure) = failure.take() {
        return Err(failure);
    }
    exited?;
    let exit = match stopped {
        Some(exit) => exit,
        None => sys::exit_of(status?),
    };
    let unread = || Kept {
        captured: Captured::default(),
        first: None,
    };
    Ok((
        exit,
        stdout.transpose()?.unwrap_or_else(unread),
        stderr.transpose()?.unwrap_or_else(unread),
    ))
}

/// Starts the readers of the child's output pipes and the writer of its
/// input, and records each in `started`.
fn start_pipes<'scope>(
    threads: &'scope Threads<'scope, '_>,
    started: &mut Started<'scope>,
    (stdin, stdout, stderr): (
        Option<std::process::ChildStdin>,
        Option<std::process::ChildStdout>,
        Option<std::process::ChildStderr>,
    ),
    notice: &'scope Notice,
    ends: &'scope Ends,
    run: &'scope Run,
) -> io::Result<()> {
    let over = || notice.set(|flags| flags.over_limit = true);
    if let Some(stdout) = stdout {
        let keeper = Keeper::new(&run.stdout);
        started.stdout = Some(
            Builder::new()
                .spawn_scoped(threads, move || pipes::read(stdout, ends, keeper, over))?,
        );
    }
    if let Some(stderr) = stderr {
        let keeper = Keeper::new(&run.stderr);
        started.stderr = Some(
            Builder::new()
                .spawn_scoped(threads, move || pipes::read(stderr, ends, keeper, over))?,
        );
    }
    if let (Some(stdin), Input::Bytes(bytes)) = (stdin, &run.stdin) {
        started.stdin =
            Some(Builder::new().spawn_scoped(threads, move || pipes::write(stdin, bytes, ends))?);
    }
    Ok(())
}

/// Waits until the child exits or the run must stop it, stops it then, and
/// returns the exit that the stop gives, or `None` when the child ended on
/// its own. The child has exited when it returns, and is unreaped.
fn control(group: &Group, notice: &Notice, run: &Run) -> Option<Exit> {
    let flags = notice.wait(run.deadline, |flags| {
        flags.exited || flags.cancelled || flags.over_limit
    });
    let exit = if flags.exited {
        return None;
    } else if flags.cancelled {
        Exit::Stopped(Stop::Cancelled)
    } else if flags.over_limit {
        Exit::Stopped(Stop::OverLimit)
    } else if group.has_exited().unwrap_or(false) {
        // The deadline passed after the child exited and before the waiter
        // said so, so the child's own exit stands.
        notice.wait(None, |flags| flags.exited);
        return None;
    } else {
        Exit::TimedOut
    };
    stop(group, notice, &run.escalation, run);
    Some(exit)
}

/// Stops the child with `escalation`, and returns once the child has exited.
/// First it tells each line queue of `run` that the run is stopping.
///
/// A signal that the operating system refuses is skipped, because the next
/// step, and in the end the kill, still applies. A grace too long for an
/// `Instant` waits without a limit.
///
/// RUST-10: the stop waits for the child's exit after the kill without a
/// limit. That handles every child that the tool may signal, and it misses
/// a child that the kill cannot reach, such as a setuid program that
/// refuses the tool's signals with `EPERM`, which holds the run until it
/// exits. A run that gives up on such a child and reports it replaces this
/// when a caller needs one.
fn stop(group: &Group, notice: &Notice, escalation: &Escalation, run: &Run) {
    run.stopping();
    if sys::GRACEFUL {
        for (signal, grace) in escalation.steps() {
            let _ = group.signal(signal);
            if notice
                .wait(Instant::now().checked_add(grace), |flags| flags.exited)
                .exited
            {
                return;
            }
        }
    }
    let _ = group.signal(Signal::Kill);
    notice.wait(None, |flags| flags.exited);
}

/// Returns a thread's result, or an error when the thread panicked, which
/// only a broken invariant in the crate can cause.
fn joined<T>(result: std::thread::Result<io::Result<T>>) -> io::Result<T> {
    result.unwrap_or_else(|_| Err(io::Error::other("a thread of the run stopped")))
}
