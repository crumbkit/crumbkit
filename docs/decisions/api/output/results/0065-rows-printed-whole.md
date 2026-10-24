# ADR-0065. Rows printed whole

- Date: 2026-10-17
- Status: amended by ADR-0072, ADR-0138 and ADR-0149
- Supersedes: ADR-0063, which it amends

## Context

Without `--full`, the JSON of rows held only the fields that
`Row::COLUMNS` and `Row::LONG_COLUMNS` name, and a field that a row left
out of its serialization printed as `null`. A tool whose scripts and agents
already read each row with every field, and with optional fields left out,
had to tell every reader to add `--full`, and the readers that did not
received a different shape.

Some rows have no fields to name. A row that serializes as an array, such
as one row of a query's answer, has no properties in its JSON Schema, so
it could not satisfy ADR-0063's rule that `Row::COLUMNS` names at least one
field.

## Decision

- `Row` gained the associated constant `WHOLE`, which is `false` by
  default. When a row type sets it, `--json` prints each row as the type
  serializes it, without `--full`, for a single row, a list, a listing, a
  batch and a stream. crumbkit refuses `--full` for such a row type with
  the reason that the JSON already holds every field.
- `--fields` still selects fields of a row type that sets `WHOLE`, in text
  and in JSON, and prints `null` for a field that a row leaves out.
  `--long` still adds the long columns to the text table.
- The compile-time checks of ADR-0063 accept a row type that sets `WHOLE`
  with an empty `Row::COLUMNS`. When both column lists are empty, the text
  prints each row as one line of compact JSON with no header, and the
  footer and notes print as for any table. crumbkit refuses `--fields` and
  `--long` for such a row type, because there is no table to shape. Help
  prints an `Output:` line that says so, and `commands list` writes the
  form `json-rows`.
- crumbkit-test's `check_guide` fails a row type that sets `WHOLE`, names
  no columns and has fields in its schema, because those fields could be
  columns. Its rule of ADR-0063 for a batch that is not headed does not
  apply to such a row type.
- crumbkit-test's `check_document_ids` fails a leaf whose row type sets
  `WHOLE`, unless an opt-out from `documents/rows-whole` with a reason
  covers it, so `check_opt_outs` reports the cost in tokens.

## Consequences

A tool keeps the row shape that its readers already parse, and the opt-out
report shows each command that pays for it. The JSON of such a command
costs more tokens than the compact columns. A row type without fields
prints in text as JSON, which a person reads less easily than a table.
`docs/behaviour/output/README.md` describes the current behaviour.

Considered: a document option in place of a row constant, which a stream
and a batch would each need separately; printing `--full` by default for
every row type; leaving out a missing field under `--fields` in place of
printing `null`; a separate text renderer that the tool supplies for rows
without columns.
