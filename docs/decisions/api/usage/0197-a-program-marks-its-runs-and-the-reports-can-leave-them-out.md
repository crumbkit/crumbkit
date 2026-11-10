# ADR-0197. A program marks its runs, and the reports can leave them out

- Date: 2026-11-10
- Status: accepted. It amends ADR-0029 with a new record field and one warning, ADR-0034 with a filter, and ADR-0079, whose crumbkit warnings all started with `warning: <tool> left out `.

## Context

A program that drives a tool, such as a page that runs `self figures list`
each time it loads the tool, adds its own runs to the log with the same
caller as a person's, and then cannot remove them from what it shows.

## Decision

- A program sets `<TOOL>_PROGRAM` to its name, spelled as
  `<TOOL>_OUTPUT_BUDGET` is (ADR-0079). The log records it in an optional
  `program` field, and the record keeps its `v1` id (ADR-0007).
- A name is 1 to 32 lowercase ASCII letters, digits, `-` and `_`, starting
  with a letter. The reader skips a record whose name fails that rule.
- The log reads the variable once, when `parse` gives it the tool's name,
  so a run that never reaches `parse` reads none. A set, non-empty value
  that fails the rule prints one `warning:` line on standard error then,
  before the tool's output and never from the signal thread. It names the
  variable and the rule but not the value, and the run's line, when one is
  written, has no field. An unset or empty variable prints nothing. The
  example checks ignore this warning, which does not start with
  `warning: <tool> left out `.
- `self usage`, `self figures list` and `self runs list` take a repeatable
  `--exclude-program <NAME>` and leave those runs out before counting.

## Consequences

The api-1 ground is one adopter's ordinary need that other programs which
drive tools share. `self runs export` prints the field as stored. The
warning is the one output of the log, because a silent misspelling would
leave the runs counted with no sign of why.

Considered: a caller value such as `program:flightdeck`, which mixes who
ran the tool with what started it; leaving marked runs out of the log; a
silent refusal; echoing the value in the warning, which may hold anything.
