# ADR-0172. What the process crate adds to its first decision

- Date: 2026-11-06
- Status: accepted
- Supersedes: ADR-0171, which it amends
- Missed: ADR-0171 did not decide what its first adopters need beyond one run in a group of its own: a child that stays in the caller's process group, the time to a child's first byte, the shape of the line queue, how long the `forward` feature waits before it kills, and on which platform a run's time bound holds.

## Context

ADR-0171 decided crumbkit-process. Building it for its first adopters
showed needs that ADR-0171 does not decide. board runs a plugin that is
called inside another plugin in its caller's process group, so the caller's
signals reach both. board also reports the time from a plugin's start to
its first byte of output. flightdeck reads a command's output line by line
while the command runs. ADR-0171 states a time bound for every run, which
Windows cannot keep, because a reader there blocks on its pipe.

## Decision

- `Scope::Child` starts the child in the caller's process group, or with
  no job object on Windows, and each signal goes to the child alone. The
  default, `Scope::Group`, is the group or job that ADR-0171 decides.
- `Finished::first_stdout` gives the time from the child's start to the
  first byte of standard output.
- The type that holds a started child and its group stays internal to the
  crate, because no adopter needs it apart from a run.
- A stream delivered as lines goes to a queue that `line_queue(most)`
  makes, which holds at most `most` lines. The run pushes each line without
  its newline, and a full queue makes the run's reader wait. Once the run
  begins to stop the child, a full queue drops each line and keeps the last
  one, which the caller reads with `Lines::lost` once the run has ended. A
  caller that drops its side lets the child run to its end. The run ends the
  queue on every return path. Each queue serves one run.
- `Captured::long_line` tells that one line passed its limit,
  `Lines::recv_timeout` waits for a line at most a given time, and it
  returns `Silent` when none arrived.
- On SIGINT, SIGTERM or SIGHUP, the `forward` feature sends the received
  signal to every listed group, waits up to `FORWARD_GRACE`, one second,
  for every listed child to exit, sends SIGKILL to every listed group, runs
  the registered clean-ups, and then takes the signal's default action. It
  holds the list's lock from the signal until the tool ends, so no child
  starts after the signal. A clean-up therefore must not start a run in
  `Scope::Group` or wait for one, and a run in `Scope::Child` takes no
  lock. The forwarder must be the only handler that ends the tool on these
  signals. A tool that also uses crumbkit-usage's signal thread, which ends
  the tool on SIGINT or SIGTERM, loses that guarantee until one owner ends
  the tool.
- Each signal other than SIGKILL, from a stop or from the forwarder, is
  followed by SIGCONT to the same processes, so a child that job control
  stopped handles it at its step.
- On Windows, `Scope::Group`'s job object replaces the command's creation
  flags.
- A run's time bound, the time to its deadline plus the graces of its
  escalation plus the drain grace, holds on Unix for a child that the tool
  may signal and a caller that reads its lines. A reader on Windows blocks
  on its pipe until the pipe closes, and a child that refuses the tool's
  signals holds the run until it exits.

## Consequences

board and flightdeck can each move their own process code to the crate.
A caller that stops reading a line queue holds a run whose child has not
been stopped. The `forward` feature cannot let a child's group outlive the
grace once the tool gets a signal.

Considered: a channel from std for the lines, which cannot drop a line
only after a stop; a public group type for callers that start a child
without a run; a longer grace for `forward`, which would hold the tool's
exit for every signal.
