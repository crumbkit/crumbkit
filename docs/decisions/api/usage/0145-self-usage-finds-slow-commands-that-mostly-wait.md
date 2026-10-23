# ADR-0145. `self usage` finds slow commands that mostly wait

- Date: 2026-10-23
- Status: accepted
- Supersedes: ADR-0034, which it amends

## Context

ADR-0034 gave `self usage` eight findings, among them `slow command`. A
builder who reads that a command is slow cannot tell whether the time goes
to the tool's own work or to a server that it waits on, and the two need
different fixes. ADR-0144 lets a tool record the time that a run waited on
another system.

## Decision

- `self usage` has a ninth finding, `mostly waiting`. It appears beside a
  `slow command` finding when some of the command's runs took at least 1
  second and spent at least half of that time waiting, and it counts those
  runs. A command whose tool records no waiting never has it.
- The report reads only `waited_ms` from a record's facts. It never reads
  the tool's counts or labels, so it never prints a name or label that a
  hand-edited line holds, and the record checks of ADR-0034 stay as they
  are.

## Consequences

The `<tool>/self-usage/v1` schema lists the new finding. A tool that adds
no facts gets the same report as before. `docs/behaviour/usage-log/report.md`
lists the current findings.

Considered: a share of time waiting among each command's figures, which
changes the `self figures list` document for one finding; a finding for
any run that waited, which flags fast commands; reading the tool's counts,
which would need checks against names that crumbkit does not know.
