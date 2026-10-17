# ADR-0075. Several run observers

- Date: 2026-10-17
- Status: accepted
- Supersedes: ADR-0057, which it amends
- Missed: ADR-0057 told a tool that wants two observers to install one that forwards to both, but the usage log's observer is private to crumbkit-usage, so a tool could not forward to it and had to choose between the log and its own observer.

## Context

ADR-0057 kept one observer per process, and the first `observe` won. A tool
that wanted crumbkit-usage's log and an observer of its own, such as one
that counts runs for its own metrics, had to write an observer that forwards
each report to both. crumbkit-usage installs its observer inside `start`
and does not export it, so that forwarding observer could not reach the
log. Exporting the log's observer, or adding a `start` that installs
nothing, would add public items to two crates for one wiring problem.

## Decision

- A process holds up to four observers in a fixed array of `OnceLock`
  slots. `observe` keeps its signature, fills the first free slot and
  returns false when all four are full or the run answers a completion
  request.
- Each report goes to every installed observer in the order they were
  installed. An observer installed after a report hears only the reports
  that follow it, as an observer installed late did before.
- crumbkit combines the observers' answers. A hand-over leaves SIGINT alone
  when any observer handles it. `trace::child_env` returns the variables of
  every observer in install order and keeps the first value of each name.
  The `tracing` spans record the ids of the first observer that gives them.
- The public `events::observer` function, which returned the one observer,
  is removed. crumbkit reads the observers through crate-private functions.
- `observe` does not check whether an observer is already installed,
  because two zero-sized statics can share an address, so comparing
  addresses could refuse a different observer. The usage log's `start`
  installs its observer only on its first call in a process.

## Consequences

A tool installs its own observer and starts the usage log in either order,
and both hear each run. A report with no observer still costs one atomic
load and a branch, and a report with observers loads one slot more than the
number installed, unless all four are full. A tool that needs a fifth
observer writes a forwarding observer, as every tool with a second observer
did before.

Considered: a provided fan-out type, which needs the usage log's observer
to be public and a `start` that installs nothing; a `Vec` behind a lock,
which takes a lock on every report; refusing `observe` after the first
report, which turns a late install into a refusal that is harder to
diagnose than a missed early report; refusing an observer whose address is
already installed; and eight slots, which no tool has needed.
