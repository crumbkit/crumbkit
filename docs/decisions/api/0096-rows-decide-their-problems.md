# ADR-0096. Rows decide their problems

- Date: 2026-10-19
- Status: accepted
- Supersedes: ADR-0050, which it amends

## Context

ADR-0050 marked a row with a problem by the field that `Row::PROBLEM`
names. The third adopting tool computes a row's status as an enum, so it
had to add a duplicate column that only repeats the status as a flag.

## Decision

- `Row::is_problem(&self)` decides whether a row reports a problem.
  crumbkit counts the rows it marks for the exit code, the note, and the
  rows that a limit cut.
- Its default reads the field that `Row::PROBLEM` names, so a row type
  that sets `PROBLEM` keeps its behaviour.
- A row type with a computed status overrides `is_problem` and leaves
  `PROBLEM` unset. Its help then prints no `Problems:` line, and
  `check_guide` checks no problem field.

## Consequences

A row type marks problems without a duplicate column. Columns that report
a problem per variant wait for a second tool that asks for them.
`docs/behaviour/output/listings/README.md` describes the current
behaviour.

Considered: letting `PROBLEM` name the status column, which failed
because help says any value in that column marks a problem and
`check_guide` refuses a field that cannot be empty.
