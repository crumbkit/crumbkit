# Processes

This page covers the crumbkit-process crate, which runs another program
whose output a tool reads: how it starts the child, what it keeps of the
child's output, how it stops the child, and how it reaps it on Unix and
Windows. The rustdoc of
[`run`](../../crates/crumbkit-process/src/run.rs) and
[`Group`](../../crates/crumbkit-process/src/group.rs) describes each item.

## The crumbkit-process crate

The published crumbkit-process crate depends on neither crumbkit nor
crumbkit-child, because it prints nothing and parses no command line. It
takes a `std::process::Command` that the tool built, so the tool still
chooses the program, its arguments and its environment. On Unix it depends
on rustix, which crumbkit already uses, and on Windows on process-wrap
([dependencies](../project/dependencies.md)).

`run` is the one-shot run. `Group` holds a started child and the processes
that it starts, and is the part of `run` that a tool can use alone to
start, signal and reap a child.

Why: ADR-0171, ADR-0003.

## Scope

By default the child leads a new process group on Unix, and runs in a new
job object on Windows. Every signal goes to the group, and a stop ends the
job. On Unix a child that moved to another process group gets each signal
too. A descendant that left the group, such as one that called `setsid`, is
not stopped.

`Scope::Child` keeps the child in the caller's process group, and each
signal goes to the child alone, so a process that the child starts is never
stopped by the crate. A tool that runs inside another tool's group uses it,
so the outer tool's signals still reach its child.

Why: ADR-0171.

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

`Captured::over_limit` tells whether the kept bytes are all of the stream.
`Finished::first_stdout` gives the time from the start to the first byte of
standard output.

On Unix each read waits on its pipe with `poll`, so every thread that the
run starts is joined, even while a descendant outside the group holds a pipe
open. On Windows a reader blocks until its pipe closes, and ending the job
closes every pipe that the job's processes held.

Why: ADR-0171.

## Ending a run

A run ends when the child exits, when its deadline passes, when its
`Cancel` is cancelled, or when a `Whole` stream with `OverLimit::Stop`
passes its limit. `Exit` tells these apart: `Code` and `Signal` for the
child's own end, `TimedOut`, and `Stopped` with `Cancelled` or `OverLimit`.
A deadline that passes after the child exited gives the child's own exit.

A stop follows the run's `Escalation`. Each step sends a signal to the
group and waits its grace for the child to exit, and a kill follows the last
step. The default escalation has no steps, so it kills at once. The signals
are interrupt, terminate, hangup and kill. On Windows a stop skips the steps
and ends the job at once, because a graceful step would send Ctrl-Break,
which needs unsafe code.

Once the child has exited, the run kills what is left of its group, reaps
the child, and reads the pipes for at most `DRAIN_GRACE`, one second, more.
A descendant outside the group that holds a pipe therefore holds the run
for at most one second after the reap, and keeps running. A run takes at
most the time to its deadline, plus the graces of its escalation, plus the
drain grace.

Why: ADR-0171.

## Reaping

On Unix the child's process id is also its group's id, and another process
can take that id once the child is reaped. The crate therefore waits for
the child with `waitid` and `WNOWAIT`, which leaves it unreaped, kills what
is left of the group, and only then reaps it. It never signals the group
after the reap. macOS refuses a signal to a group whose only process has
exited, and the crate counts that refusal as sent.

A `Group` that is dropped before its reap kills every process in its scope
and reaps the child, so a normal return and a panic that unwinds leave
nothing running. On Windows the drop ends the job. A tool that is killed
outright or crashes skips the drop: on Unix the group keeps running, and on
Windows the job's processes keep running, because process-wrap's std job
object does not ask Windows to end them when the job's last handle closes.

Why: ADR-0171.
