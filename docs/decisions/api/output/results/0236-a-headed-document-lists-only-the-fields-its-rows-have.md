# ADR-0236. A headed document lists only the fields its rows have

- Date: 2026-11-16
- Status: accepted
- Supersedes: ADR-0062, which it amends

## Context

ADR-0062 made a headed document's `--help`, its `commands list` row and its
`--fields` hint always name `text` and `base64`. A headed batch whose row
type is `#[serde(transparent)]` over a JSON value has a schema with no
properties. Its `--help` still said that `--fields` selects `text` and
`base64`, its `-h` still ended with `Next: ... --help explains each JSON
field.`, and `--fields text` was refused with the hint `use the valid fields: `,
which lists nothing.

## Decision

- A headed document's column list ends with `text` and `base64` for each
  that is a property of the row schema, as `row::properties` reads it.
  A headed document that names no column has JSON fields to list, and so
  prints the `Next:` line and the `JSON fields` section, only when the
  schema has `text` or `base64`.
- `--fields` on a leaf whose row schema has no properties is refused with
  `--fields names the field 'text', and this command takes no --fields,
  because its rows have no named fields`, the hint `drop --fields`, the code
  `<tool>::cli::usage` and exit 2. `parse` and `finish` print the same
  refusal.
- crumbkit-test's `check_help` fails a headed leaf whose row type has a field
  whose schema holds both a `text` and a `base64` property, which is a
  `Bytes` field without `#[serde(flatten)]`, and names the field. A row type
  with no properties passes.

## Consequences

Help for a headed leaf builds the row schema once more to decide whether to
print the `Next:` line. That happens only while help is rendered and only for
a headed leaf that names no column. A leaf whose rows have no properties
still offers `--fields`, because the leaf chooses its view flags from
constants and reading the schema to hide the flag would cost a schema build
on every run.

Considered: hiding `--fields` from such a leaf, which needs the schema build
on every run.
