# ADR-0173. How the process crate waits on a stopped child and signals a moved one

- Date: 2026-11-06
- Status: accepted
- Supersedes: ADR-0172, which it amends
- Missed: ADR-0172 did not decide how the wait for a child's exit handles a child that job control stopped and then continued, or whether a refused signal to the group stops the signal to a child that moved to another group.

## Context

On macOS, a `waitid` for an exit with `WNOWAIT` also returns a report that
the child stopped, for as long as the child stays stopped. `WNOWAIT` leaves
that report in place, so each later wait returns it again at once. The
crate's wait therefore woke every 10 ms for as long as a child stayed
stopped, which a child stopped by job control may do for hours. A review
asked whether a report that the child continued could do the same; on the
macOS that the crate was tested on, a wait for an exit never returns one.

The same review found that a refused signal to the child's group returned
before the crate signalled a child that moved to another group. macOS
refuses a signal to a group whose only process has exited, so a child that
left an unreaped process behind in its old group got no signal.

## Decision

- The wait for an exit treats every report other than an exit alike,
  whether it says that the child stopped or continued. It consumes the
  report with a second wait for stops and continues, which leaves an exit
  in place, and then waits again, so the kernel holds the wait until the
  next report. When it finds no report to consume, it waits 10 ms before
  it checks again.
- A signal to the group that the kernel refuses still lets a child that
  moved to another group get the signal and the SIGCONT that follows it.
  The first refusal is the result.

## Consequences

The wait wakes once for each stop of the child, and never polls while a
stopped child stays stopped. Another wait in the tool for the child's
stops or continues misses each report that the crate consumed. A kqueue
`EVFILT_PROC` filter with `NOTE_EXIT` replaces the wait if the tool needs
those reports. A child that moved gets the escalation's signals even when
its old group holds no live process.

Considered: a kqueue `NOTE_EXIT` filter on macOS now, which needs a second
wait path for one platform; a bound on how long the wait polls, which still
wakes every 10 ms until that bound.
