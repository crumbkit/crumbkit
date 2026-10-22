# ADR-0140. The `Rows:` line names every view that lists every row

- Date: 2026-10-22
- Status: accepted
- Supersedes: ADR-0138, which it amends
- Missed: ADR-0138 made `--fields` list every row of a listing that lists only its problem rows, but its `Rows:` line named only `--long` and `--json`, and an adopter's reviewer found that a reader of the help could not learn what `--fields` lists.

## Context

A leaf whose document lists only its problem rows prints a `Rows:` line
in `-h` and `--help`. Under ADR-0138 the line read `Rows: the text lists
only the rows that report a problem, and --long or --json lists every
row.` An adopting tool's reviewer reported two gaps. The line does not
say that `--fields` lists every row, although it does. The help of the
`--long` flag itself says only that it adds the long columns, so a reader
who looks at the flag's help does not learn that `--long` also lists
every row on this leaf.

## Decision

- The `Rows:` line reads `Rows: the default text lists only the rows that
  report a problem, and --long, --fields or --json lists every row.` It
  names `--long` and `--fields` only when the leaf offers them, so a row
  type that sets `Row::WHOLE` and names no columns reads `... and --json
  lists every row.`
- "The default text" replaces "the text", because the text that `--long`
  and `--fields` print lists every row as well.
- The `Rows:` line carries what `--long` does on this leaf, and the help
  of the `--long` flag stays the same on every leaf. The `Rows:` line
  prints in `-h` and `--help`, while a flag's description prints only in
  `--help`. One line then says which rows each view lists, and a global
  flag's help does not change from one leaf to another. ADR-0139 kept the
  `--long` help generic for the same reason.

## Consequences

A reader of `-h` learns from one line which views list every row of the
listing. A reader who reads only the `--long` flag's description in
`--help` still sees only the long columns there, and has to read the
`Rows:` line below the columns. `docs/behaviour/output/listings/README.md`
describes the current behaviour.

Considered: replacing the help of `--long` on a leaf that lists only its
problem rows with a sentence about every row, which makes a global flag's
help differ between leaves and prints only in `--help`; and a second line
for `--fields`, which repeats the subject of the `Rows:` line.
