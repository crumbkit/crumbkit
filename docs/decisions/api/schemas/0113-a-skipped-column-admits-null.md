# ADR-0113. A skipped column admits null

- Date: 2026-10-20
- Status: accepted, amended by ADR-0132
- Supersedes: ADR-0110, which it amends
- Missed: ADR-0110 cut the required list and left the type of a skipped column unable to admit the null that the compact document prints.

## Context

ADR-0110 cut a row's `required` list to its columns. The compact document
prints `null` for a column that the row type skips when it is empty, and
the column's property kept a type without null, so that document failed
the schema. An adopting tool added a null type to such a field by hand.

## Decision

- When the cut keeps a column that the row did not require and the row's
  properties hold, the column's property admits null. A `type` gains
  `"null"`. A property without a plain `type` becomes an `anyOf` of itself
  and `{"type": "null"}`, with its description beside the `anyOf`, as
  schemars writes `Option<T>`.
- The column stays optional, because `--json --full` leaves it out.
- The change happens where the cut does: in each enum variant, and in a
  definition or its inline copy.
- A property that already admits null keeps its schema.

## Consequences

`docs/behaviour/schemas/README.md` states the rule, and the RUST-10
comment in `crates/crumbkit/src/schemas/columns.rs` keeps only the
untagged-enum limit.

Considered: requiring every column with null admitted, which makes the
`--json --full` document fail because it leaves the field out; leaving
the tool to add a null type by hand, which puts an output fact on the
type.
