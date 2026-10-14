# ADR-0034. Usage report

- Date: 2026-10-11
- Status: accepted

## Context

The usage log holds one line per run, which a builder cannot read by eye
across thousands of runs. The builder needs the patterns where callers
waste effort, per command, without the report itself leaking what a
hand-edited line puts in the log.

## Decision

- `self usage` read the log once, a line at a time, holding at most one
  record's bytes, so a large log costs little memory.
- A run counted as failed on exit 1 and as refused on the guide's refusal
  code. Other tool codes count as neither, because their meaning belongs
  to the tool.
- The report had eight findings, each with a fixed threshold, such as
  `one id at a time`, `help loop` and `heavy output`.
- Each record was checked against the tool's command tree and spelling
  rules before it counts, and a failing record is counted as skipped, so
  a hand-edited line cannot put its text in the report.
- The output was a listing of findings with a summary, and `--full` adds
  each command's figures. `self figures list` printed every command's
  figures under a `figures` resource, so it fits the grammar.
- A missing or unreadable log reads as empty and exits 0, as every
  usage-log failure stays silent.

## Consequences

`cargo crumbkit weigh --usage` reads `self figures list`.
`docs/behaviour/usage-log/report.md` lists the current findings and
thresholds.

Considered: counting every exit other than 0 and 2 as a failure, a summary as an extra row, a separate summary document beside the listing, reading the log into memory, a report that fails on a missing log, a check of codes against the predefined spellings only, a check of codes against the tool's own spelling, a `--since` in days only, a hidden `self` group, every command's figures in the summary of `self usage`, a `self usage figures` subcommand, which would make `usage` a resource.
