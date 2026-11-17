# ADR-0237. A run can report at a time and keep waiting for the child

- Date: 2026-11-17
- Status: accepted

## Context

`RunOptions::deadline` always stops the child, and an `Escalation` always
ends with a kill. A tool that must never signal its child, such as one that
reclaims stuck jobs or counts the processes of other tools, still needs to
learn that the child has outlived a time limit. Each such tool kept a
hand-written wait loop beside the crate to do it.

A tool that needs to report a child that outlives a time limit and let it
go on is an ordinary need that other tools are likely to share, so one
adopter's request meets API-1 on the third ground of ADR-0122.

## Decision

`RunOptions::overdue(at, report)` makes the run call `report` once, on the
run's own thread, when `at` passes while the child still runs. `report`
receives an `Overdue` that gives the child's process id and the time since
the child started. The run then goes on waiting, obeys its cancel, its
limits and its deadline as before, and never signals the child because `at`
passed.

`ChildFinished::was_overdue` says whether `report` ran. `Outcome` keeps its
variants, so a child that exits by itself after the report gives
`Ended(status)`.

`report` never runs when the child has exited by `at` or when `at` is at or
after the deadline. A panic in `report` stops the child, reaps it, and
returns `RunError::Run`, as a panic in a copy function does.

## Consequences

The run does nothing else while `report` runs, so a `report` that blocks
delays the run's reaction to a cancel, an over-limit stream and the
deadline. The documentation says so.

Considered: `RunOptions::deadline_without_stop(at)` with an
`Outcome::DeadlinePassed`, which would hide whether the child ended by
itself or was stopped; an `Escalation` that never kills, which would also
leave a cancelled child running; repeated reports and a report that returns
a decision, which no adopter needs, and a report that wants to stop the run
calls `Cancel::cancel`.
