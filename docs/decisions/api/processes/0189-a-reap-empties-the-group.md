# ADR-0189. A reap waits until the child's group is empty

- Date: 2026-11-10
- Status: accepted
- Supersedes: ADR-0171 and ADR-0172, which it amends
- Missed: ADR-0171 sent one kill to the group before the reap and assumed that it ended every process in the group, and ADR-0172's time bound had no time for the processes that the kill ends.

## Context

ADR-0171 decided that the crate kills what is left of the child's group
once and then reaps the child. A run therefore returned while the processes
that the kill ended were still in the group, until their new parent reaped
them. On macOS a kill of a group can also miss a process that is being
forked, and that process keeps running.

## Decision

- Before the reap, the crate kills the group again for as long as a live
  process is left in it. Only macOS checks here, with signal 0, because
  Linux delivers a kill of a group to a process that is being forked.
- After the reap, the crate checks the group with signal 0, which delivers
  nothing, until the kernel finds no process in it. It never sends another
  signal after the reap. The group's id cannot pass to another process while
  a process is in the group, so the check names no other group.
- Both waits end `REAP_GRACE`, one second, after the child's exit. The reap
  then returns the child's status and leaves the processes that are still in
  the group.
- A run's time bound adds `REAP_GRACE`.

## Consequences

A run that returns leaves no process in the child's group, unless a process
outlasts `REAP_GRACE`, as an orphan under an init process that reaps nothing
does. The report of
survivors from `RunOptions::report_survivors` is taken before the first kill,
so it does not change.

Considered: one kill and no wait, which left the ended processes in the
group when the run returned; repeating the kill after the reap, which could
signal a group that took the id after the group emptied; a check of `/proc`
on Linux before the reap, which reads every process on each run; no bound,
which lets a process that is never reaped hold the run.
