//! The one-shot run: start a child, write its input, read its output within
//! limits and a deadline, stop it when needed, and reap it.

use std::fmt;
use std::fs::File;
use std::io;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::thread::{Builder, Scope as Threads, ScopedJoinHandle};
use std::time::{Duration, Instant};

use zeroize::Zeroizing;

use crate::notice::Notice;
use crate::{Cancel, Escalation, Group, Outcome, Scope, Signal, Status, StopReason, sys};

mod finished;
mod output;
mod pipes;
mod spawn;

#[cfg(test)]
mod tests;

pub use self::finished::{ChildFinished, RunError};
pub use self::output::{Capture, Captured, LongLine, OverLimit};
use self::output::{Keeper, OutputCopy};
use self::pipes::{Ends, Kept};
use self::spawn::AroundSpawn;
pub use self::spawn::{Spawn, Spawned};

/// How long a run keeps reading the child's pipes after it reaps the
/// child. A descendant outside the child's scope that holds a pipe open
/// holds the run no longer than this.
pub const DRAIN_GRACE: Duration = Duration::from_secs(1);

/// How long after the child exits a run on Unix waits for the child's group
/// to be empty. On macOS the run also kills the group again while a live
/// process is left. A process of the group that outlasts it is left behind
/// and the run goes on to read the pipes: one that waits for a parent that
/// never reaps it, or a live one that the tool may not signal, such as a
/// setuid program.
pub const REAP_GRACE: Duration = Duration::from_secs(1);

/// What the child reads on its standard input.
#[derive(Clone, Default)]
#[non_exhaustive]
pub enum Input {
    /// The null device, so the child reads the end of its input at once.
    #[default]
    Null,
    /// These bytes, which a thread of the run writes and then closes. The
    /// thread stops early when the child closes its input.
    ///
    /// The buffer is cleared when it drops, so a secret given to a child
    /// does not stay in the tool's memory after the settings are gone.
    /// Each clone of the settings holds and clears its own copy. The
    /// clearing covers this buffer only: it misses copies that the caller
    /// made before and the pipe's buffer in the kernel. `Debug` prints the
    /// length alone.
    Bytes(Zeroizing<Vec<u8>>),
    /// The tool's own standard input. On Unix a child in a group of its own
    /// that reads a terminal is stopped by SIGTTIN, so a terminal suits
    /// only [`Scope::Child`].
    Inherit,
    /// A file or the read end of a pipe, which a pipe becomes with
    /// `File::from(OwnedFd::from(reader))` on Unix and
    /// `File::from(OwnedHandle::from(reader))` on Windows. The child gets a duplicate that
    /// shares the file's position, so it reads from the current position
    /// and moves the position that the caller and any later run see. The
    /// caller's `File` stays open.
    File(Arc<File>),
}

impl fmt::Debug for Input {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => f.write_str("Null"),
            Self::Bytes(bytes) => f
                .debug_tuple("Bytes")
                .field(&format_args!("{} bytes", bytes.len()))
                .finish(),
            Self::Inherit => f.write_str("Inherit"),
            Self::File(file) => f.debug_tuple("File").field(file).finish(),
        }
    }
}

/// The settings of one [`run`]: the child's input, what it keeps of each
/// output stream, when and how it stops the child, and the child's scope.
///
/// A `RunOptions` value that delivers a stream as lines serves one run,
/// because the run ends the line queue when it returns.
///
/// The default reads nothing, keeps nothing, has no deadline and no cancel,
/// kills at once when it stops the child, and starts the child in a group
/// of its own.
#[derive(Clone, Debug, Default)]
pub struct RunOptions {
    stdin: Input,
    stdout: Capture,
    stderr: Capture,
    deadline: Option<Instant>,
    cancel: Option<Cancel>,
    escalation: Escalation,
    scope: Scope,
    around_spawn: Option<AroundSpawn>,
    stdout_copy: Option<OutputCopy>,
    stderr_copy: Option<OutputCopy>,
    report_survivors: bool,
}

impl RunOptions {
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
    /// which then ends as [`Outcome::TimedOut`].
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

    /// Makes `hook` start the child, so the caller can hold a lock or change
    /// process-wide state for exactly the length of the start.
    ///
    /// The run calls `hook` once, on the thread that called [`run`], and
    /// never when its cancel was already cancelled. The hook does its work
    /// before the start, calls [`Spawn::start`], does its work after the
    /// start, and returns the [`Spawned`] that the start gave. The hook
    /// surrounds the whole start, including the `forward` feature's lock.
    ///
    /// A hook that returns an error ends the run with [`RunError::Start`].
    /// When the child had started, dropping its [`Spawned`] kills the child
    /// and every process in its scope and reaps the child. A panic in the
    /// hook does the same as it unwinds.
    ///
    /// The time the hook takes counts towards the deadline, and it is not
    /// part of the time bound that [`run`] states. Time that the hook spends
    /// after the start counts towards
    /// [`first_stdout`](ChildFinished::first_stdout), because the run reads
    /// the child's output only once the hook returns. A hook that waits for a
    /// lock can check the deadline itself and return an error. With the
    /// `forward` feature, a start waits forever once the tool has received
    /// a signal, so a clean-up must not wait for a lock that the hook holds.
    ///
    /// ```no_run
    /// use std::io;
    /// use std::process::Command;
    /// use std::sync::Mutex;
    ///
    /// use crumbkit_process::{RunOptions, run};
    ///
    /// static FORK_GATE: Mutex<()> = Mutex::new(());
    ///
    /// fn inherit(descriptors: bool) -> io::Result<()> {
    ///     // Clears or sets CLOEXEC on the descriptors the child inherits.
    ///     # let _ = descriptors;
    ///     Ok(())
    /// }
    ///
    /// let options = RunOptions::new().around_spawn(|spawn| {
    ///     let _gate = FORK_GATE.lock().map_err(|_| io::Error::other("poisoned"))?;
    ///     inherit(true)?;
    ///     let spawned = spawn.start();
    ///     inherit(false)?;
    ///     spawned
    /// });
    /// # let _ = run(Command::new("true"), &options);
    /// ```
    #[must_use]
    pub fn around_spawn(
        mut self,
        hook: impl Fn(Spawn) -> io::Result<Spawned> + Send + Sync + 'static,
    ) -> Self {
        self.around_spawn = Some(AroundSpawn::new(hook));
        self
    }

    /// Calls `copy` with each chunk of standard output as the run reads it,
    /// whatever the capture keeps, so a tool can show the output while the
    /// run keeps a bounded copy. See [`RunOptions::stderr_copy`].
    #[must_use]
    pub fn stdout_copy(mut self, copy: impl Fn(&[u8]) + Send + Sync + 'static) -> Self {
        self.stdout_copy = Some(OutputCopy::new(copy));
        self
    }

    /// Calls `copy` with each chunk of standard error as the run reads it,
    /// whatever the capture keeps.
    ///
    /// A chunk can end in the middle of a line or of a UTF-8 character. The
    /// run does not read a stream with [`Capture::Inherit`], so its function
    /// receives nothing.
    /// Each stream's function runs on that stream's reader thread, so the
    /// two streams can interleave. A function that blocks holds the reader:
    /// the child then waits on its full pipe, and the run waits for the
    /// function past the time bound that [`run`] states. A function that
    /// panics gets no more chunks, the run reads the rest of the stream so
    /// the child runs on, and the run ends with [`RunError::Run`].
    #[must_use]
    pub fn stderr_copy(mut self, copy: impl Fn(&[u8]) + Send + Sync + 'static) -> Self {
        self.stderr_copy = Some(OutputCopy::new(copy));
        self
    }

    /// Makes the run check, when the child ends by itself and before the run
    /// kills what is left of the group, whether any other process in the
    /// group still runs. [`ChildFinished::survivors`] gives the answer. The
    /// run kills those processes as it always does.
    ///
    /// The check is off by default because on Linux it reads every process
    /// in `/proc`. It reports on [`Scope::Group`] on Linux and macOS alone.
    #[must_use]
    pub const fn report_survivors(mut self) -> Self {
        self.report_survivors = true;
        self
    }

    /// Tells the queue of each stream delivered as lines that the run began
    /// to stop its child.
    fn stopping(&self) {
        self.stdout.stopping();
        self.stderr.stopping();
    }

    /// Tells the queue of each stream delivered as lines that the child has
    /// exited, so a full queue drops lines once `deadline` has passed.
    fn drain_until(&self, deadline: Instant) {
        self.stdout.drain_until(deadline);
        self.stderr.drain_until(deadline);
    }
}

/// Runs `command` with the settings in `options` and returns how it ended and
/// what it printed.
///
/// The run sets the command's standard streams, starts it in the scope of
/// [`RunOptions::scope`], and waits until the child exits, the deadline passes,
/// the cancel is cancelled, or a stream whose policy stops the child passes
/// its limit. In the last three cases it stops the child with the
/// [`Escalation`]. Once the child has exited, the run kills what is left of
/// its group, reaps it, waits on Unix until the group is empty for at most
/// [`REAP_GRACE`] after the exit, and reads its pipes for at most
/// [`DRAIN_GRACE`] more. A run whose deadline passes after the child exited gives the
/// child's own exit.
///
/// On Unix, for a child that the tool may signal, a run takes at most the
/// time to its deadline, plus the graces of its escalation, plus
/// [`REAP_GRACE`], plus [`DRAIN_GRACE`], plus the time that a hook of
/// [`RunOptions::around_spawn`] takes, plus the time that a function of
/// [`RunOptions::stdout_copy`] or [`RunOptions::stderr_copy`] blocks. Every
/// thread that it starts is joined before it returns. On Windows a stop ends the job at once, and a reader waits for
/// its pipe to close, which ending the job causes for every process in the
/// job, so a process outside the job that holds a pipe holds the run.
///
/// A stream delivered as lines waits for the caller while its queue is
/// full, and the child then waits on its full pipe. Once the run begins to
/// stop the child, a full queue drops each line instead. After the child
/// exits by itself, a full queue waits for the caller until the drain
/// deadline, half of [`DRAIN_GRACE`] after the reap, and then drops each
/// line while the reader goes on reading the pipe until [`DRAIN_GRACE`]
/// after the reap. [`Lines::lost`](crate::Lines::lost) says which line a
/// caller can rely on. The run ends each line queue when it returns, on
/// every path.
///
/// # Errors
///
/// Returns [`RunError::Start`] when the program cannot be started, a hook
/// of [`RunOptions::around_spawn`] fails, or an [`Input::File`] cannot be
/// duplicated, and
/// [`RunError::Run`] when waiting for, reaping or reading the child fails
/// or a copy function panics.
/// The child is stopped and reaped on every path.
pub fn run(mut command: Command, options: &RunOptions) -> Result<ChildFinished, RunError> {
    let _end = EndLines(options);
    if options.cancel.as_ref().is_some_and(Cancel::is_cancelled) {
        return Ok(ChildFinished {
            outcome: Outcome::Stopped(StopReason::Cancelled),
            status: None,
            stdout: Captured::default(),
            stderr: Captured::default(),
            first_stdout: None,
            survivors: false,
        });
    }
    let input = match &options.stdin {
        Input::Null => Stdio::null(),
        Input::Inherit => Stdio::inherit(),
        Input::Bytes(_) => Stdio::piped(),
        Input::File(file) => Stdio::from(file.try_clone().map_err(RunError::Start)?),
    };
    command
        .stdin(input)
        .stdout(output_stdio(&options.stdout, options.stdout_copy.as_ref()))
        .stderr(output_stdio(&options.stderr, options.stderr_copy.as_ref()));
    let spawned = spawn::start(command, options.scope, options.around_spawn.as_ref())
        .map_err(RunError::Start)?;
    let (mut group, started) = spawned.into_parts();
    let notice = Arc::new(Notice::default());
    if let Some(cancel) = &options.cancel {
        cancel.watch(&notice);
    }
    let ends = Ends::default();
    let followed = std::thread::scope(|threads| {
        let unwind = Unwind {
            group: &mut group,
            ends: &ends,
            options,
        };
        follow(threads, &mut *unwind.group, &notice, &ends, options)
    });
    let (outcome, status, stdout, stderr, survivors) = followed.map_err(RunError::Run)?;
    Ok(ChildFinished {
        outcome,
        status: Some(status),
        first_stdout: stdout.first.map(|first| first - started),
        survivors,
        stdout: stdout.captured,
        stderr: stderr.captured,
    })
}

/// Returns the standard stream for an output with `capture` and `copy`. A
/// copy needs a pipe even when the capture keeps nothing. An inherited
/// stream stays the tool's own, so a copy of it receives nothing.
fn output_stdio(capture: &Capture, copy: Option<&OutputCopy>) -> Stdio {
    if copy.is_some() && !matches!(capture, Capture::Inherit) {
        Stdio::piped()
    } else {
        capture.stdio()
    }
}

/// Ends the line queues of a run when the run returns, on every path.
struct EndLines<'run>(&'run RunOptions);

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
    options: &'a RunOptions,
}

impl Drop for Unwind<'_> {
    fn drop(&mut self) {
        // A reaped group refuses the signal, so the kill never reaches a
        // process id that another process may have taken.
        let _ = self.group.signal(Signal::Kill);
        self.ends.set(Instant::now());
        self.options.stopping();
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
/// its exit, its reaped status and what the readers kept.
fn follow<'scope>(
    threads: &'scope Threads<'scope, '_>,
    group: &mut Group,
    notice: &'scope Notice,
    ends: &'scope Ends,
    options: &'scope RunOptions,
) -> io::Result<(Outcome, Status, Kept, Kept, bool)> {
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
    let mut failure = start_pipes(threads, &mut started, pipes, notice, ends, options).err();
    let stopped = if failure.is_some() {
        stop(group, notice, &Escalation::default(), options);
        None
    } else {
        control(group, notice, options)
    };
    let survivors =
        options.report_survivors && failure.is_none() && stopped.is_none() && group.others_alive();
    let status = group.reap();
    // The child is gone. The queues drop lines from the middle of the drain,
    // so the reader still has time to read the rest of the pipe and `lost`
    // can be the stream's last line.
    let reaped = Instant::now();
    let after = |grace: Duration| reaped.checked_add(grace).unwrap_or(reaped);
    ends.set(after(DRAIN_GRACE));
    options.drain_until(after(DRAIN_GRACE / 2));
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
    let status = sys::status_of(status?);
    let exit = stopped.unwrap_or(Outcome::Ended(status));
    let unread = || Kept {
        captured: Captured::default(),
        first: None,
    };
    Ok((
        exit,
        status,
        stdout.transpose()?.unwrap_or_else(unread),
        stderr.transpose()?.unwrap_or_else(unread),
        survivors,
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
    options: &'scope RunOptions,
) -> io::Result<()> {
    let over = || notice.set(|flags| flags.over_limit = true);
    if let Some(stdout) = stdout {
        let keeper = Keeper::new(&options.stdout);
        let copy = options.stdout_copy.as_ref();
        started.stdout = Some(Builder::new().spawn_scoped(threads, move || {
            pipes::read(stdout, ends, keeper, copy, over)
        })?);
    }
    if let Some(stderr) = stderr {
        let keeper = Keeper::new(&options.stderr);
        let copy = options.stderr_copy.as_ref();
        started.stderr = Some(Builder::new().spawn_scoped(threads, move || {
            pipes::read(stderr, ends, keeper, copy, over)
        })?);
    }
    if let (Some(stdin), Input::Bytes(bytes)) = (stdin, &options.stdin) {
        started.stdin =
            Some(Builder::new().spawn_scoped(threads, move || pipes::write(stdin, bytes, ends))?);
    }
    Ok(())
}

/// Waits until the child exits or the run must stop it, stops it then, and
/// returns the exit that the stop gives, or `None` when the child ended on
/// its own. The child has exited when it returns, and is unreaped.
fn control(group: &Group, notice: &Notice, options: &RunOptions) -> Option<Outcome> {
    let flags = notice.wait(options.deadline, |flags| {
        flags.exited || flags.cancelled || flags.over_limit
    });
    let exit = if flags.exited {
        return None;
    } else if flags.cancelled {
        Outcome::Stopped(StopReason::Cancelled)
    } else if flags.over_limit {
        Outcome::Stopped(StopReason::OverLimit)
    } else if group.has_exited().unwrap_or(false) {
        // The deadline passed after the child exited and before the waiter
        // said so, so the child's own exit stands.
        notice.wait(None, |flags| flags.exited);
        return None;
    } else {
        Outcome::TimedOut
    };
    stop(group, notice, &options.escalation, options);
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
fn stop(group: &Group, notice: &Notice, escalation: &Escalation, options: &RunOptions) {
    options.stopping();
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
