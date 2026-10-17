# ADR-0070. Every answer left out keeps its failure

- Date: 2026-10-17
- Status: accepted
- Supersedes: ADR-0068 and ADR-0046, which it amends

## Context

A batch marked with `Document::left_out_per_key` fell back to the one
failure of ADR-0046 when its first row and the failure of every later
answer passed the output budget (ADR-0068). The readers that the mark is
for match each failure to its key and cannot read the one failure, so a
batch with large rows broke them at the point where the budget mattered.

The one failure had a fault of its own. When the first row and a failure
that named no key already passed the budget, the failure named no key and
counted the keys under `unlisted`, and its recovery became a hint. The
document passed the budget either way, so dropping the keys saved a few
bytes and lost the recovery.

## Decision

- A batch marked with `left_out_per_key` always gives each answer left out
  its own failure. When the first row and those failures pass the budget,
  the first row prints with every failure, and the JSON document passes the
  budget by the size of those failures. The fallback to the one failure is
  removed.
- When the first row and a failure that names no key pass the budget, the
  one failure of a batch without the mark names every key left out, and
  its recovery runs the command with them.
- The `left-out-per-key` customisation that `check_opt_outs` lists says
  that the document can pass the budget by the size of the failures.

## Consequences

A reader that matches each failure to its key finds every key answered once
whatever the size of the rows. The cost is bytes: about 200 for each answer
left out, so a selector such as `--all` over 10,000 keys can print about
2 MB past a 1 MiB budget. The tool's opt-out report states this cost.

Considered: per-key failures up to a ceiling such as twice the budget with
the one failure above it, printing no row and every failure, and a
`Document::key_field(name)` that puts each per-key failure's key under a
field that the tool names. A tool that wants the key under its own field
name writes a failure form, which already reads `PreparedFailure::key`.
