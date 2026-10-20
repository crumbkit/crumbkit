# ADR-0110. Schemas describe what the tool prints

- Date: 2026-10-20
- Status: accepted
- Supersedes: ADR-0007, which it amends

## Context

ADR-0007 derived each document's schema from its types with schemars,
which describes what deserialises unless its settings say otherwise. A
field with `skip_serializing_if` and no serde default was then required,
although the document leaves it out when it is empty. A tool added
`#[serde(default)]` to such fields only to make the schema match its
output. The schema also required every field of a row, while `--json`
without `--full` prints only the row's columns, so only the `--full`
document matched the schema.

## Decision

- Every schema of a document that the tool prints uses one set of
  settings: draft 2020-12 with schemars' serialize contract. A field that
  serde skips when it is empty is optional, and a field that serde always
  prints is required, whether or not it has a serde default.
- A file format's schema keeps the deserialize contract, because users
  write the file and the tool reads it.
- In a document with the envelope whose JSON prints the row's columns, the
  row's `required` list keeps only the names in `Row::COLUMNS`, so the
  documents of `--json`, `--json --long` and `--json --full` all match the
  schema. An enum row has each variant's list cut. A row whose definition
  serves another field is cut in an inline copy. A row type that sets
  `Row::WHOLE`, a headed batch, a document without the envelope, prose and
  bytes keep every field required.

## Consequences

`docs/behaviour/schemas/README.md` states the rule, and
`docs/behaviour/output/README.md` says which views match the schema. The
checked-in schemas mark fewer row fields as required, and fields that print
`null` are required.

Considered: a `serde(default)` on every skipped field, which puts a
deserialisation fact on a type to fix a schema; a hand-written `required`
list per document, which drifts from the type; two schemas per document,
compact and full, which doubles `schemas list` and gives one id two shapes.
