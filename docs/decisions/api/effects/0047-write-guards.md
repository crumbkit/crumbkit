# ADR-0047. Write guards

- Date: 2026-10-12
- Status: accepted

## Context

An agent that retries a write, or that writes while another caller
writes, needs to know two things that the effects of ADR-0039 do not say:
whether the command replaces data that another caller may have changed,
and what a failed write of many items leaves behind.

## Decision

- A leaf declared both on its `Effects`. `Effects::replacing` marks a
  command that replaces existing data as destructive and not idempotent,
  because a second run can overwrite a change made between the two runs.
  `all_or_none` and `per_item` say what a failed write of many items
  leaves.
- Help, `commands list` and the Markdown reference printed the guards.
  `commands list` holds them under `writes`, with `null` when there are
  none, so the row keeps its `v1` id.
- An `Expect` argument added `--expect <VERSION>`, with `none` reserved
  for a target that must not exist. The tool decides what a version is.
  `Expect::check` runs before `confirm`, so a conflict comes before the
  question, and a conflict fails with `crumbkit::version::conflict`,
  whose recovery runs the tool's read command.
- `check_effects` required `--expect` on a replacing leaf and a declared
  item guard on a writing leaf that takes many positional values, each
  with an opt-out. It reads a positional argument as the items, because
  options with many values more often filter or describe the change. The
  rules that a read-only leaf takes no guard and a `per_item` leaf prints
  a document that reports each item have no opt-out.

## Consequences

A replacing command still takes `--yes`, because it is destructive.
`docs/behaviour/effects/write-guards.md` describes the current behaviour.

Considered: a separate guards type beside `Effects`, guards as MCP annotations of their own, a `takes_expect` declaration on the leaf, a required `--expect`, an `If-Match` style header name for the flag, an `expect` reason under the `confirmation` area, a `writes` field only in a `v2` row, `--yes` skipped when `--expect` is given, no rule for positional arguments with many values.
