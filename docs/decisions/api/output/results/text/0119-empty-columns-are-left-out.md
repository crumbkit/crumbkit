# ADR-0119. Empty columns are left out of the text table

- Date: 2026-10-20
- Status: accepted

## Context

An adopting tool lists records of two kinds in one table. A column such as
a state applies to one kind, so a record of the other kind prints `-` in
it. When a page holds only the other kind, the column is `-` in every row
and its width carries no information.

## Decision

- When the text table that `finish` prints shows at least one row, it
  leaves out a column whose field is `null` or missing in every shown row,
  header included. The output budget can cut the shown rows short.
- crumbkit decides from each serialized field, so a string that holds `-`
  keeps its column.
- The rule applies to the compact columns and to `--long`. With `--fields`
  every named column prints, and `--json` is unchanged.
- A stream prints every column, because its header prints before any row.
- The problem field is a column like any other.

## Consequences

`docs/behaviour/output/README.md` states the rule. `Room` still counts a
left-out column, so a batch's handler can stop before the row at which
`finish` would cut.

Considered: a `Document` or `Row` option naming columns to leave out,
which one tool would set; deciding from the cell text, which drops a
column of strings that hold `-`; applying the rule to `--fields`, which
hides a column the caller named.
