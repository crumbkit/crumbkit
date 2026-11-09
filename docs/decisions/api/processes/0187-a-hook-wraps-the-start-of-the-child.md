# ADR-0187. A run's hook wraps the start of its child

- Date: 2026-11-09
- Status: accepted
- Supersedes: ADR-0172, which it amends
- Missed: ADR-0172 did not decide whether a caller can act around the start of a child, or whether the time such a caller takes counts in a run's time bound.

## Context

`run` starts the child itself, so a tool has no way to act at the moment of
the fork. A tool that passes a descriptor to a child has to clear CLOEXEC on
that descriptor before the fork and set it again after. While CLOEXEC is
clear, another thread's fork would leak the descriptor, so the tool also
holds a lock that every fork in the process takes. Restoring CLOEXEC can
fail, and a tool that cannot restore it must not keep the child it started.

## Decision

- `RunOptions::around_spawn` takes a function that receives a `Spawn`, calls
  `Spawn::start` once, and returns the `Spawned` that the start gave.
- `Spawned` has no public constructor, so a hook that returns `Ok` has
  started the child. It has no public methods either, because the type that
  holds a started child stays internal (ADR-0172).
- A hook's error ends the run with `RunError::Start`. Dropping a `Spawned`
  kills the child and its scope and reaps it, which is what a `Group` does
  when it drops.
- The time to the first byte is counted from `Spawn::start`, so a hook that
  waits for a lock before the start does not lengthen it. The run reads the
  child's output only once the hook returns, so time the hook spends after
  the start does count.
- The time a hook takes is outside the time bound of a run that ADR-0172
  states.
- The hook runs in `run`, so it works the same on Unix and Windows and
  surrounds the `forward` feature's lock.

## Consequences

A tool can take a lock, change descriptor flags and report a failed
restoration without the crate knowing about either. The hook is a function
stored in `RunOptions`, so it needs `Send + Sync + 'static`. The time a hook
takes is outside a run's time bound, and a hook that waits for a lock
checks the deadline itself.

Considered: a hook that returns a guard the run drops after the start, which
has only `Drop` in which to restore and cannot report a failure, and whose
`'static` guard cannot borrow a lock that the hook's closure owns; separate
before and after callbacks, which cannot hold a lock in a local variable
between them; a `Spawn::start` that returns nothing, which lets a hook skip
the start and return success.
