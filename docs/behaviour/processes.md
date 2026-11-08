# Processes

This page covers the crumbkit-process crate, which runs another program
whose output a tool reads: how it starts the child, what it keeps of the
child's output, how it stops the child, and how it reaps it on Unix and
Windows. The rustdoc of
[`run`](../../crates/crumbkit-process/src/run.rs),
[`line_queue`](../../crates/crumbkit-process/src/lines.rs) and
[`forward`](../../crates/crumbkit-process/src/forward.rs) describes each
item.

## The crumbkit-process crate

The published crumbkit-process crate depends on neither crumbkit nor
crumbkit-child, because it prints nothing and parses no command line. It
takes a `std::process::Command` that the tool built, so the tool still
chooses the program, its arguments and its environment. On Unix it depends
on rustix, which crumbkit already uses, and on Windows on process-wrap
([dependencies](../project/dependencies.md)).

`run` is the one-shot run. The type that holds a started child and its
group is internal to the crate.

Why: ADR-0171, ADR-0172, ADR-0003.

## Scope

By default the child leads a new process group on Unix, and runs in a new
job object on Windows. Every signal goes to the group, and a stop ends the
job. On Unix a child that moved to another process group gets each signal
too, and the SIGCONT that follows it, even when its old group refuses the
signal. A descendant that left the group, such as one that called
`setsid`, is not stopped.

On Windows the job object replaces the command's creation flags with its
own, which start the child suspended, so flags that the tool set with
`creation_flags` are lost.

`Scope::Child` keeps the child in the caller's process group, and each
signal goes to the child alone, so a process that the child starts is never
stopped by the crate. A tool that runs inside another tool's group uses it,
so the outer tool's signals still reach its child.

Why: ADR-0171, ADR-0172, ADR-0173.

## Input and output

The child's standard input is the null device, bytes that a thread writes
and then closes, or the tool's own input. The writer stops early, with no
failure, when the child closes its input.

Each output stream has a `Capture`:

- `Discard` sends the stream to the null device.
- `Whole` keeps the stream's first `limit` bytes. A longer stream is over
  its limit, and `OverLimit` decides what happens then: `Stop` stops the
  child, and `Sink` reads the rest of the stream and drops it, so the child
  runs to its end.
- `Tail` reads the whole stream and keeps its last `limit` bytes.
- `Lines` splits the stream on each newline and pushes each line, without
  its newline, to a queue from `line_queue` while the child runs. A last
  line with no newline is pushed when the stream ends. The first time one
  line passes `line` bytes or the stream passes `total` bytes, the stream
  is over its limit, and `OverLimit` decides what happens as for `Whole`.
  No line is pushed after that.

`Captured::over_limit` tells whether the kept bytes, or the lines
delivered, are all of the stream, and `Captured::long_line` tells whether
one line passed its limit. `ChildFinished::first_stdout` gives the time from the
start to the first byte of standard output.

The caller takes each line with `Lines::recv` or `Lines::recv_timeout`,
which return `None` once the run has ended and the queue is empty. The run
ends the queue on every return path, including a start failure and a
cancel before the start. A full queue makes the run's reader wait, and the
child then waits on its full pipe. Once the run begins to stop the child,
a full queue drops each line at once. Once the child has exited by itself,
a full queue waits for the caller until half of `DRAIN_GRACE` has passed
since the reap, and then drops each line while the reader goes on reading
the pipe until `DRAIN_GRACE` after the reap. `Lines::lost` gives the last
dropped line after the run ends. It is the stream's last line when the
reader reached the end of the pipe. Three cases leave it as another line.
A process outside the child's group that holds the pipe open and goes on
writing, or a reader too slow to reach the end within the second half of
the grace, leaves it as the last line read when the reads ended, which can
be part of a line. A stream over its limit stops pushing lines while the
reader goes on, so `lost` is the last line dropped before the limit, and
`Captured::over_limit` shows that case. A later queued line clears `lost`,
so `None` does not show that no line was dropped. A caller that
drops its `Lines` lets the run read the rest of the stream and drop it, so
the child runs to its end. Each queue serves one run, so a `Limits` value
with a `Lines` capture serves one run too.

On Unix each read waits on its pipe with `poll`, so every thread that the
run starts is joined, even while a descendant outside the group holds a pipe
open. On Windows a reader blocks until its pipe closes, and ending the job
closes every pipe that the job's processes held.

Why: ADR-0171, ADR-0176.

## Ending a run

A run ends when the child exits, when its deadline passes, when its
`Cancel` is cancelled, or when a `Whole` or `Lines` stream with
`OverLimit::Stop` passes its limit. `Outcome` tells these apart: `Code` and
`Signal` for the child's own end, `TimedOut`, and `Stopped` with
`Cancelled` or `OverLimit`.
A deadline that passes after the child exited gives the child's own exit.

`ChildFinished::status` gives the child's own status as the run reaped it,
which a timed-out or stopped run has too: `Status::Code` with the exit
code, or `Status::Signal` with the signal's number on Unix. It is the
status that the stop caused, so a cancelled child that SIGINT ended gives
`Signal(2)`, and one that handles SIGINT and exits 7 gives `Code(7)`. On
Windows a stopped child gives `Code(1)`. `Status::shell_code` gives the
code that a POSIX shell gives, 128 plus the signal's number for a signal,
so the first gives 130. A run whose cancel was cancelled before it started
a child has no status.

A stop follows the run's `Escalation`. Each step sends a signal to the
group and waits its grace for the child to exit, and a kill follows the
last step. The default escalation has no steps, so it kills at once. The
signals are interrupt, terminate, hangup and kill. On Unix each signal
other than kill is followed by SIGCONT, so a child that job control
stopped, as SIGTTIN stops one that reads the terminal, handles the signal
at its step. On Windows a stop skips the steps and ends the job at once,
because a graceful step would send Ctrl-Break, which needs unsafe code.

A grace too long for the clock waits for the child's exit without a limit.

Once the child has exited, the run kills what is left of its group, reaps
the child, and reads the pipes for at most `DRAIN_GRACE`, one second, more.
A descendant outside the group that holds a pipe therefore holds the run
for at most one second after the reap, and keeps running.

On Unix, for a child that the tool may signal, a run takes at most the
time to its deadline, plus the graces of its escalation, plus the drain
grace. A child that refuses the tool's signals, such as a setuid program,
holds the run until it exits. On Windows a reader blocks until its pipe
closes, so a process outside the job that holds a pipe, as a grandchild of
a child in `Scope::Child` can, holds the run until it exits.

Why: ADR-0171, ADR-0172, ADR-0174, ADR-0176, ADR-0178.

## Reaping

On Unix the child's process id is also its group's id, and another process
can take that id once the child is reaped. The crate therefore waits for
the child with `waitid` and `WNOWAIT`, which leaves it unreaped, kills what
is left of the group, and only then reaps it. It never signals the group
after the reap. macOS refuses a signal to a group whose only process has
exited, and the crate counts that refusal as sent.

macOS also gives a wait for an exit a report that the child stopped, for
as long as it stays stopped, because `WNOWAIT` leaves that report in place.
The wait treats every report other than an exit alike, whether it says
stopped or continued: it consumes the report with a second wait for stops
and continues, which leaves an exit in place, and then waits again. It
therefore wakes once for each stop, and never polls while a stopped child
stays stopped. When it finds no report to consume, it checks again after
10 ms. Another wait in the tool for the child's stops or continues misses
each report that the crate consumed.

A run that panics or returns early still kills every process in its scope
and reaps the child, so a normal return and a panic that unwinds leave
nothing running. On Windows it ends the job. A tool that is killed
outright or crashes skips this: on Unix the group keeps running, and on
Windows the job's processes keep running, because process-wrap's std job
object does not ask Windows to end them when the job's last handle closes.

Why: ADR-0171, ADR-0173.

## Forwarding the tool's signals

The off-by-default `forward` feature exists on Unix only. `forward::install`
starts one thread for the life of the process, and `forward::clean_up`
registers a function to run before the tool ends. A run lists its child
when the child leads a group of its own, and unlists it before the reap,
both under one lock, so the forwarder never signals a reaped id.

When the tool receives SIGINT, SIGTERM or SIGHUP, the thread takes the
lock and holds it until the tool ends, so no child starts after the
signal. It sends the received signal, and then SIGCONT, to every listed
group and to a listed child that moved to another group, waits up to
`FORWARD_GRACE`, one second, for every listed child to exit, sends SIGKILL
to every listed group, runs each clean-up, and then takes the signal's
default action. A clean-up that panics does not stop the others.

A clean-up must not start a run in `Scope::Group`, or wait for a thread
that runs one, because that run waits forever for the lock that the
forwarder holds. A run in `Scope::Child` takes no lock.

The forwarder must be the only handler that ends the tool on SIGINT,
SIGTERM or SIGHUP. crumbkit-usage's signal thread ends the tool on SIGINT
or SIGTERM, and can do so before the forwarder has stopped the groups, so
a tool that uses both loses that guarantee until one owner ends the tool.

Why: ADR-0171, ADR-0172.
