# ADR-0174. A run that stopped its child keeps the child's status

- Date: 2026-11-06
- Status: accepted
- Supersedes: ADR-0172, which it amends
- Missed: ADR-0172 did not decide whether a run that stopped its child reports the status that the child ended with, which a caller needs to report a stopped child's exit.

## Context

A run that its deadline, its cancel or an output limit stopped returns
`Exit::TimedOut` or `Exit::Stopped`, and the crate dropped the status it
got when it reaped the child. A web runner reports how a cancelled run's
child ended, and a child that SIGINT ended should be reported as 130, as
a shell reports it. With the status dropped, the runner can only assume
how the child ended.

## Decision

- `Finished::status` returns the child's own status as the run reaped it,
  as a `Status`: `Code` with the exit code, or `Signal` with the number of
  the signal that ended the child, on Unix only. A run that stopped the
  child gives the status that the stop caused. It is `None` only for a run
  whose cancel was cancelled before it started a child.
- `Status::shell_code` returns the exit code, or 128 plus the signal's
  number, which is the code a POSIX shell gives.
- `Exit` keeps its variants, so a caller that matches on how the run ended
  is unchanged.
- A run that stopped its child and then failed to reap it returns
  `RunError::Run`, as a run whose child exited on its own does. Before, the
  stopped run dropped the failure with the status.

## Consequences

A caller reports a cancelled child's exit with
`finished.status().map(Status::shell_code)`. On Unix that is 130 for a
child that SIGINT ended, and the child's own code for a child that handles
the signal. On Windows a stop ends the job, so the status of a stopped
child is `Code(1)`. A child that exited on its own has its status twice,
in `Exit` and in `Status`.

Considered: `Exit::Stopped` and `Exit::TimedOut` carrying the status,
which changes every match on `Exit` and repeats one field across two
variants; a `Status` without `shell_code`, which leaves each caller to
write the shell's rule.
