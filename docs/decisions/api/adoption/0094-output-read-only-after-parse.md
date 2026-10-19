# ADR-0094. A run's Output is read-only after parse, and finish consumes it

- Date: 2026-10-19
- Status: accepted
- Supersedes: ADR-0076 and ADR-0015, which it amends
- Missed: ADR-0076 made `parse` return the `Output`, but left it mutable through the handler, so the handler had to record `--dry-run` itself with `Output::dry_run`, and `finish` had to guard against a second call with a `spent` count that lowered `Output::budget` after each result.

## Context

`parse` read `--no-input` and the view flags by their ids from clap's
matches, but a handler that flattened `DryRun` read the flag from its own
arguments and passed it to `Output::dry_run`, which recorded the dry run in
the `Output` and returned it. A command that forgot the call confirmed and
changed data on a dry run, and `finish` marked nothing.

`finish`, `finish_to`, `stream` and `stream_to` took `&mut Output`, because
`finish` builds the branch of the command tree that a next step or a
recovery names (ADR-0015). The `Output` therefore outlived the call, so a
`main` could call `finish` twice with one `Output`. `Output` kept a `spent`
count of the bytes that `finish` had printed, and `Output::budget` returned
the budget less that count, so that a second result printed within what
was left. `write_result` took the command out of the `Output` with
`std::mem::take` and put it back after, because its writer borrowed the
other fields while a failure rendered its call against the command. A
`Streaming` borrowed the view and the command from the `Output` for its
whole life.

## Decision

- `parse` reads `--dry-run` by its id `crumbkit_dry_run` from the deepest
  command's matches, as it reads `--no-input` and the view flags, and
  records it in the `Output`. `Output::is_dry_run` reports it, and
  `Output::confirm` asks nothing on a dry run. `Output::dry_run` is
  removed. `Output::new` and `Output::new_to` take the command's `DryRun`
  after the view flags, and a tool without the flag passes
  `DryRun::default()`. `DryRun` keeps its place as the flag that a leaf
  flattens, so help, `check_effects` and the schema are unchanged.
- `finish`, `finish_to`, `stream` and `stream_to` take the `Output` by
  value. A `Streaming` owns the view and the command tree that it renders
  against, and has no lifetime parameter. A second `finish` with one
  `Output` does not compile, which a `compile_fail` doctest on `finish`
  shows. crumbkit-child's `hand_over` and `hand_over_json` take the
  `Output` by value for the same reason.
- `spent` is removed, and `Output::budget` returns the whole budget, which
  a handler reads before `finish` to fetch no more rows than print.
  `Streaming::budget` still reports what a stream has left.
- `write_result` borrows the settings of the run apart from the command
  tree, so the take and the put-back go.
- `Output::command`, `commands::list`, `schemas::list`, `schemas::show`,
  `Room` and `Output::confirm` still take `&mut Output`, and each runs
  before `finish`. The rest of ADR-0076 and ADR-0015 is unchanged.

## Consequences

A handler reads the dry run from the `Output` that `parse` gave it, and a
tool cannot confirm on a dry run by forgetting a call. A tool's `main`
calls `finish` once per leaf, as the last call of the run, and the compiler
refuses a second call. A tool that called `finish` more than once with one
`Output`, or that called `Output::dry_run`, changes its `main`, which is a
breaking change. `Output::new` gains a parameter, so every tool that
parses with clap alone adds `DryRun::default()` or its own flag.

`docs/behaviour/effects/README.md` describes how `parse` records the dry
run, `docs/behaviour/output/README.md` and `docs/behaviour/output/streams.md`
describe the by-value calls, and the adoption and parsing pages describe
`main`. The rustdoc of `DryRun`, `Output`, `Output::new`, `finish`,
`finish_to`, `stream`, `stream_to` and `Streaming` says the same. The
changelog records the changed signatures.

Considered: a typestate `Output<Parsed>` and `Output<Finished>`, rejected
because the only transition is the last call, which consumption by value
expresses with no type parameter; keeping `spent` and documenting that
`finish` is called once, rejected because the compiler can refuse the
second call; keeping `Output::dry_run` beside the parsed flag, rejected
because two sources of one fact let them disagree.
