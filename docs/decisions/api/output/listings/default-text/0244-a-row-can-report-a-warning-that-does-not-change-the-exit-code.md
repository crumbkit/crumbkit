# ADR-0244. A row can report a warning that does not change the exit code

- Date: 2026-11-17
- Status: accepted
- Amends: ADR-0157 and ADR-0212
- Missed: ADR-0212 let a row mark itself only as a problem, and a problem always exits with 1, so a tool whose rows deserve attention without failing the run had no mark for them.

## Context

`Row::PROBLEM` is the only way a row marks itself, and a run whose rows
report a problem exits with 1. An adopting tool lists entries where some
deserve attention but must not fail the run. It counts those rows by hand
and writes the count into its listing's note, so crumbkit neither keeps such
rows in the default text nor tells a tool where to sort them.

## Decision

- `Row::WARNING: Option<Warning<Self>>` names the rows that report a
  warning. `Warning` has the shape of `Problem`: `Warning::field` marks a
  row whose field serializes to a value other than `null`, `false` or an
  empty string, and `Warning::by` lets a function decide. Both types wrap
  one private rule.
- `RowReports::reports_warning` is true when `WARNING` marks the row and
  `PROBLEM` does not, so a row counts once, as its worse state.
- The exit code depends only on problems. A run with warnings, no problems
  and no failure exits with 0.
- The text prints `note: N of the <description> report a warning.` on
  standard error after the problem note and before the tool's note, when the
  count is above 0 and the run has no failure. `finish`, a headed batch and a
  stream print it, and `Listing::from_all` counts the rows its limit cuts.
- The JSON document gains nothing. The row's own column holds the state.
- The default text lists a row that reports a warning whatever
  `Row::in_default_text` returns, so the note never counts a row the text
  hides. `Document::only_problems` lists the rows that report a problem or a
  warning, and the `Rows:` help line says so.
- `Warning::field` adds a `Warnings:` help line after the `Problems:` line.
  `Warning::by` adds none, as `Problem::by` adds none.
- OUTPUT-3 lists rows that report a problem first, then rows that report a
  warning, then the rest.
- `check_help` fails a warning column that is not one of `COLUMNS`, a
  `Warning::field` field whose type has no empty value, and a
  `Warning::field` that names the field of `Problem::field`.
- `#[row(warning)]` in the `Row` derive writes `Warning::field`.

## Consequences

A tool that sorted problem rows first adds the warning key to its sort. No
in-repo row sets `WARNING`, so no output changes for an existing tool, except
that the `Rows:` line of a document that lists only its problem rows now
reads "report a problem or a warning".

Considered: a `Style::Warning` cell style, which a tool can write with
`cell_style`; a `warnings` count in the JSON document, which problems have
none of either; a `Warnings:` help line for `Warning::by`, which would name
no field; counting a row that reports both as a problem and a warning, which
would make the two notes add up to more than the rows.
