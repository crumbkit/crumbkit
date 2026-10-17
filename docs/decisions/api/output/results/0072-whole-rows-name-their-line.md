# ADR-0072. Whole rows without columns name their line

- Date: 2026-10-17
- Status: accepted
- Supersedes: ADR-0065 and ADR-0063, which it amends
- Missed: ADR-0065 gave a whole row type without columns a text form of its own, with its own help line and `commands list` form, although ADR-0064 had added `Row::LINE` for rows that print their own line on the same day. ADR-0063 did not ask how the text of such a row would read.

## Context

Under ADR-0065, a row type that set `Row::WHOLE` and named no columns
printed each row as one line of JSON in text. Help printed its own
`Output:` line for it, and `commands list` wrote the form `json-rows`.
A row type that declares `Row::LINE` and returns `None` from
`Row::text_line` prints the same JSON line, so crumbkit kept two ways to
reach one text form.

## Decision

- A row type that sets `WHOLE` may name no columns only when it also
  declares `Row::LINE`. `finish` fails to compile for a single row, a
  `Vec` or a `Listing` of a whole row type without columns and without a
  `LINE`, and `stream` fails to compile for such a row type.
- The compiler cannot check a batch, so crumbkit-test's `check_guide`
  fails a leaf whose whole row type has no columns and no `LINE`.
- crumbkit removes the `json-rows` form of `commands list`, its help line
  and the text form that printed each row as JSON without a `LINE`.
- `Output::room` counts the rows of a row type that declares `LINE` as the
  lines that `finish` prints.

## Consequences

A tool whose rows have no fields declares the form of their text line, and
help prints that form on its `Text:` line. A row that has no line of its
own still prints as one line of compact JSON. The rest of ADR-0065 is
unchanged.

Considered: keeping the JSON-lines form as the default for a whole row type
without columns; ignoring `WHOLE` on such a row type.
