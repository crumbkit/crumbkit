# ADR-0171. The process crate

- Date: 2026-11-05
- Status: accepted

## Context

Some tools run another program and read what it prints. flightdeck, for
example, runs a crumbkit tool's commands and draws their results. Each
such tool solves the same problems: it starts the child so that the child's
descendants can be stopped with it, reads standard output and standard
error within byte limits and a deadline, stops the child gracefully before
it kills it, and reaps it. Each tool that did this wrote its own code, and
that code ran only on Unix.

The order of reaping and signalling matters. A child that leads its own
process group gives the group its process id. Once the child is reaped,
that id can pass to another process, and a later signal to the group can
reach processes that the tool never started.

crumbkit-child does not cover this case. It hands the terminal to the child
and never reads the child's output (ADR-0058).

## Decision

- A published crate, crumbkit-process, runs a child whose output the tool
  reads. It depends on neither crumbkit nor crumbkit-child, because it
  prints nothing and parses no command line.
- It takes a `std::process::Command` that the tool built, so the tool's own
  code still chooses the program, its arguments and its environment.
- On Unix the child leads a new process group. On Windows the child runs in
  a job object, so ending the job ends the child and every descendant that
  stays in the job. Standard input is null, bytes that a thread writes, or
  inherited.
- The caller gives each output stream one of four policies: discard it,
  keep all of it up to a limit, keep its last bytes up to a limit, or
  deliver it as lines through a bounded queue with a limit on each line and
  on the total. When a stream passes its limit, the run either stops the
  child or reads the rest into a sink, as the caller chooses.
- On Unix a read with a deadline waits with `poll`, so every reader thread
  ends and is joined, even while a descendant that left the group holds a
  pipe open. On Windows, whose pipes do not support `poll`, a reader thread
  blocks on its pipe until the pipe closes, and ending the job closes it.
- A stop follows an escalation, which is a list of steps. Each step sends a
  signal to the group and waits a grace period, and the last step kills.
  The default escalation only kills. Each caller states its own steps.
- When the child exits or a stop ends, the crate waits for the child
  without reaping it, kills what is left of its group, and only then reaps
  it. When the child has moved to another group, each signal also goes to
  the child itself. After the reap the crate reads the pipes for a short
  drain grace and then closes them.
- A run ends with an exit code, a signal, a timeout or a stop, and the
  caller can tell these apart.
- An off-by-default `forward` feature stops every live group when the tool
  itself receives SIGINT, SIGTERM or SIGHUP, runs the clean-up functions
  that the tool registered, and then takes the signal's default action.
  The feature exists only on Unix.
- On Windows a stop ends the job at once, with no graceful step. A graceful
  step would send Ctrl-Break with `GenerateConsoleCtrlEvent`, which no
  maintained crate wraps without unsafe code, and crumbkit has no unsafe
  code (ADR-0003).
- On Unix the crate uses rustix. On Windows it uses the job object of
  process-wrap, which starts the child suspended and adds it to the job
  before it runs, so no process that the child starts escapes the job.
- process-wrap's std API does not ask Windows to end a job's processes when
  the job's last handle closes. The crate therefore ends the job when a run
  is dropped, which covers a normal return and a panic that unwinds. When
  the tool is killed outright or crashes, the job's processes keep running.
  This gap closes when process-wrap's std API can ask for kill-on-close.
- Windows stays tier 2 (ADR-0003). The crate's Windows tests run in the
  scheduled Windows job.

## Consequences

A tool runs such a child through crumbkit-process and keeps only the code
that is its own, such as the arguments of each operation and the meaning of
the child's output. On Windows a stopped child can leave partial work, as a
child killed with SIGKILL can on Unix, and a tool that is killed outright
leaves its children running until process-wrap offers kill-on-close. A descendant that leaves the process
group on Unix is not stopped. The crate does not supervise programs that
outlive the tool, restart them or recognise them across restarts.

Considered: process-wrap, whose Unix group wrapper reaps the group's leader
before it signals the group; subprocess, which has no job objects;
win32job, which adds the child to its job only once the child runs, so a
process that the child starts at once can escape the job; a module
in crumbkit-child, which would bring crumbkit's output types to a runner
that prints nothing; unsafe code in one Windows module for Ctrl-Break;
reader threads on Unix, which cannot be joined while a descendant holds a
pipe; one fixed stop sequence for every caller; a crate that also
supervises long-running programs.
