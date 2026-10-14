# ADR-0057. Run observer

- Date: 2026-10-14
- Status: accepted

## Context

The usage log needed to hear what each run did from many places in
crumbkit, some of which have no guide, such as `finish`, a tolerated
error, crumbkit-aliases and the usage log's own signal thread. Wiring each
place to the usage log with its own `cfg` attribute made every build
compile different code. Other observers, such as a tool's own metrics,
need the same events.

## Decision

- An `events` module with no cargo feature held an `Observer` trait, an
  `observe` function and the reports, so every build compiles the same
  reports and a call site has no `cfg`. The usage log became one
  observer.
- One observer was kept in a process-wide `OnceLock`, because the reports
  take no guide. A tool that wants two observers installs one that
  forwards to both.
- `observe` installed nothing for a completion request, so no observer
  hears a Tab press.
- The observer also answered whether it handles SIGINT, which variables
  put a child in the run's trace, and the ids for the `tracing` spans,
  because a hand-over and the spans need those answers.
- Every method had a default body, and `Refused` and `Finished` are
  `#[non_exhaustive]` with getters, so crumbkit can add events and fields
  in a minor release.
- A report with no observer costs one atomic load and a branch, and work
  that only an observer needs happens after that check. `finish_to`
  always counts its bytes, because counting only with an observer would
  need two copies of the response path.

## Consequences

The usage log no longer has call sites spread through crumbkit.
`docs/behaviour/run-observer/README.md` lists the current events.

Considered: an observer on `Guide`, which `ended`, `tolerated`, `aliased`, the signal thread and `child_env` cannot reach; a list of observers; a public `completing` function that each observer checks; an `events` cargo feature that compiles the reports out; counting bytes only when an observer is installed, which needs two copies of the response path.
