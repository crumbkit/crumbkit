# ADR-0234. The usage record carries lock wait and context

- Date: 2026-11-16
- Status: accepted
- Supersedes: ADR-0029 and ADR-0145, which it amends

## Context

Some facts belong to every tool. A tool that holds a state lock has a time
that it waited to take the lock, and a tool that finds a project root has a
place that it ran in, such as inside a project or outside one. The only home
for a tool's facts is the `facts` section (ADR-0144), which the log leaves
out whenever the line would pass 1 KiB, so a fact that `self usage` relies on
cannot live there. A wait on a lock is also not a wait on another system, and
adding it to `waited_ms` would hide which of the two slowed the run.

## Decision

- The record gains two optional fields beside its other core fields, and
  keeps the `crumbkit/usage/v1` id, because an optional new field leaves the
  id as ADR-0007 says. A reader that ignores unknown fields reads old and new
  lines alike.
- `lock_ms` is the milliseconds that the run spent waiting to take locks,
  summed. It is left out when it is 0, and a sum that would pass `u64::MAX`
  stays there.
- `context` is one `Label` (ADR-0144) that says where the run took place.
  Several calls in one run keep the last label.
- `crumbkit_usage::lock_waited(time)` adds to `lock_ms`, and
  `crumbkit_usage::context(label)` sets `context`. Both do nothing without a
  started log. A tool calls `lock_waited(lock.waited())` for each lock it
  takes. crumbkit-usage installs no hook in crumbkit-durable, because a hook
  would also report the usage log's own lock and would need a global in a
  crate that has none.
- The log never leaves out `lock_ms` or `context` to fit the line. They add
  about 35 bytes, such as `"lock_ms":1234,"context":"member"`.
- `self usage` counts a run of at least 1,000 ms as mostly waiting when
  `lock_ms` plus `waited_ms` is at least half its wall time. The finding's
  suggestion names locks when most of the counted runs waited at least as long
  on locks as on other systems, and another system otherwise.
- The report reads `context` only when it is a `Label`, and counts any other
  value as a skipped record, so a hand-edited line cannot put its text in a
  report.
- `self runs list` gains the long columns `lock_ms` and `context`. `self runs
  list` and `self figures list` gain a repeatable `--context <LABEL>` flag,
  which keeps only the runs that reported one of the named contexts and
  leaves out the others before counting anything, as `--exclude-program` does
  for programs.

## Consequences

A tool that reports neither field writes the same lines as before. A tool that
reports them adds up to 108 bytes to a line. Old lines read as 0 and none.

Considered: a `usage/v2` id, which forces every reader to change for two
optional fields; adding the lock wait to `waited_ms`, which needs no schema
change and loses the cause; a list of contexts, which costs line bytes for a
run that has one place.
