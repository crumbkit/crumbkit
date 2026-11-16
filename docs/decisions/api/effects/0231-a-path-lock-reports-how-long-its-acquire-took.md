# ADR-0231. A path lock reports how long its acquire took

- Date: 2026-11-16
- Status: accepted
- Supersedes: ADR-0222, which it amends
- Missed: ADR-0222 gave `PathLock` no way to report the time a caller spent waiting for it, so a tool that records its own waiting had to time every acquire itself.

## Context

A tool that records how a run spent its time wants the milliseconds that
the run waited to take locks. `PathLock::acquire` already measures its own
wait for `LockError::Busy`, and the lock that it returns on success carries
no such figure, so each tool would wrap every acquire in a timer.

## Decision

- `PathLock::waited` returns the `Duration` from the start of the acquire
  that made the lock to the moment it held the lock. The time includes the
  open and each reopen that ADR-0222 describes.
- crumbkit-durable adds no other behaviour and keeps no dependency on a
  crumbkit crate. A tool passes the figure on to whatever records it.

## Consequences

A caller reads the wait of a lock it already holds at no further cost
beyond one stored `Duration`. A lock that returned `Busy` has no figure,
and `Busy` keeps its own `waited` field.

Considered: a global hook that crumbkit-durable calls on every acquire,
which would need a global in a crate that has none and would report locks
that the caller does not want counted; a timer around `acquire` in each
tool, which every tool would write the same way.
