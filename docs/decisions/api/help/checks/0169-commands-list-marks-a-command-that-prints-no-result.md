# ADR-0169. commands list marks a command that prints no result

- Date: 2026-11-04
- Status: accepted
- Supersedes: ADR-0143, which it amends

## Context

ADR-0143 gave the `commands list` row of a leaf declared with
`Leaf::prints_no_result` the form `null`, the same form as the row of a
leaf without a document, because a new value of `form` would change the
published schema. The two commands differ in the view flags they take. The
first refuses `--fields`, `--long` and `--full`, and the second offers all
three. A client that builds a form for each command from `commands list`,
such as flightdeck, could not tell them apart. It left out `--full` for
both, so a command without a document whose handler prints rows that are
not whole printed only their columns.

The versions rule in `docs/behaviour/schemas/README.md` keeps a document's
id when the change adds an optional field, because readers ignore unknown
fields. A new value of `form` needs a new id.

## Decision

- The row of a leaf that declares `prints_no_result` has a field
  `prints_no_result` whose value is `true`.
- Every other row leaves the field out, as a row without goal headings
  leaves out `goals`. The schema describes the field as optional, with the
  constant `true`, so the document keeps the id `<tool>/commands-list/v1`.
- `form` stays `null` for such a row, as ADR-0143 decided.

## Consequences

`docs/behaviour/help/commands-list.md` describes the field. A client reads
the field to decide whether a command with the form `null` takes the view
flags.

Considered: a new `form` value such as `none`, which needs a new document
id; a `prints_no_result` field on every row with `false` for most, which
ADR-0168 rejected for an empty `goals` on every row, because a reader then
reads a key on every row that only a few rows need; a field that names the
view flags each command offers, which is more than a client needs to tell
the two cases apart.
