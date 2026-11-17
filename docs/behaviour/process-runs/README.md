# A run's lifecycle

A `crumbkit_process::run` follows one child from its start to its reap.
`RunOptions::overdue(at, report)` adds one state, overdue, in which the run
has reported a child that outlived a time and goes on waiting.

States:

- **not started**
- **running**: the child runs, unreaped.
- **overdue**: `report` returned, and the child still runs, unreaped.
- **stopping**: the escalation is under way.
- **exited**: the child exited, and on Unix it stays unreaped through `WNOWAIT`.
- **reaped**
- **returned**

| From | Event | To | Effect |
| --- | --- | --- | --- |
| not started | cancel already cancelled | returned | `Stopped(Cancelled)`, no status |
| not started | spawn | running | |
| running | `at` passes, child alive | overdue | `report(Overdue)` runs once on the run's thread; no signal is sent |
| running, overdue | child exits | exited | |
| running, overdue | deadline passes | stopping | outcome `TimedOut` |
| running, overdue | cancel or over limit | stopping | outcome `Stopped(..)` |
| stopping | child exits | exited | |
| exited | group killed, child reaped | reaped | the group empties within `REAP_GRACE` |
| reaped | pipes drained | returned | within `DRAIN_GRACE`; `was_overdue` says whether `report` ran |

Crash points:

- `report` panics: the run catches the panic, kills the group, reaps, and
  returns `RunError::Run`.
- `report` blocks: the run acts on cancel, over-limit and the deadline only
  after `report` returns. The waiter still records the exit, and the reap
  follows.
- The child exits as `at` passes: the run checks the exit first. If the child
  exits between that check and the call, `report` receives the id of an
  exited, unreaped child, and the id stays reserved until the reap.
- The tool is killed while the run is overdue: on Unix the processes of the
  group keep running, and on Windows the processes of the job keep running, as
  they do for a run with no overdue time.

Invariants:

1. The run never signals the child because the overdue time passed.
2. `report` runs at most once, and only before the reap, so its id names the
   run's own child.
3. The run reaps the child on every path.
4. `Outcome` keeps its meaning, and only `ChildFinished::was_overdue` records
   the report.
5. An overdue time at or after the deadline never calls `report`.

Why: ADR-0237.
