# ADR-0058. The child crate

- Date: 2026-10-14
- Status: accepted
- Supersedes: ADR-0042, which it amends

## Context

The hand-over to a child program sat in crumbkit's `child` module behind
the off-by-default `hand-over` feature (ADR-0042). The usage log had
already moved from a crumbkit feature to the crumbkit-usage crate
(ADR-0029). The hand-over was the last crumbkit feature that added signal
handling, and it needed its own lint and test runs on Linux and Windows,
because only a build with `hand-over` alone installs the child's SIGINT
handler without the usage log.

The hand-over printed its failures and refusals with crate-private
`Output` methods and reported to the run observer through crate-private
`events` functions, so a separate crate could not print or report as
`finish` does.

## Decision

- The hand-over became the published crumbkit-child crate, which depends
  on crumbkit and signal-hook. crumbkit lost the `hand-over` feature, the
  `child` module and the `codes::CHILD_*` constants.
- crumbkit made public the `Output` methods that print a failure or refuse
  the view flags as `finish` does and that give a failure's exit code, as
  well as `Sink::standard` and the hand-over reports in `events`. Any crate
  that prints without `finish` can use them.
- The child codes kept their spelling, `crumbkit::child::*`, so a tool's
  renames and listed exits stayed valid. crumbkit-child lists them in its
  own `codes::ALL`.
- crumbkit-test treats every code in the `crumbkit` component as one of
  crumbkit's codes, so an add-on crate's code needs no twin or declaration,
  and the recovery check follows its recoveries.
- CI dropped the separate `hand-over` lint and test runs. The workspace
  runs cover crumbkit-child, and its tests run without the usage log.

## Consequences

A tool that hands the terminal over adds a dependency on crumbkit-child in
place of a feature. The public `Output` methods are part of crumbkit's API,
so a change to them needs a minor release. A code in the `crumbkit`
component that no crate raises would also pass crumbkit-test's checks
without a twin. `docs/behaviour/output/child-processes.md` describes the
current behaviour.

Considered: a `crumbkit::child` module that re-exports the new crate,
codes in a `crumbkit-child` component, a trait for printing that the
child crate implements, keeping the feature and its CI runs, a declared
twin for each child code in every tool.
