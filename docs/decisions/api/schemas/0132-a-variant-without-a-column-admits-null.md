# ADR-0132. A variant without a column admits null

- Date: 2026-10-21
- Status: accepted
- Supersedes: ADR-0113, which it amends
- Missed: ADR-0113 admitted null only for a property that the row schema declares, so a variant without the property refused the null that crumbkit prints.

## Context

An adopting tool's row type is an enum with a full variant and a short
variant. The full variant has the field that `Row::CHILDREN` names, and
the short variant has no such field. Without `--full`, the compact
document holds every column and the children field on every row, with
`null` where the row has no value (ADR-0110, ADR-0120), so a short row
prints the children field as `null`. ADR-0113 admitted null only for a
property that the row's `properties` holds. The short variant's schema
had `additionalProperties: false` and no property for the field, so it
refused the document that crumbkit printed. A column that one variant has
no field for failed in the same way.

## Decision

- The printed JSON stays as it was. Every compact row holds every column
  and the children field, with `null` where the row has no value.
- When the cut reaches a row schema or a variant whose `properties` lacks
  a name in the columns or the children field, the schema gains the
  property `{"type": "null"}` for that name.
- Only a schema with a `properties` map, or with
  `additionalProperties: false`, gains the property. A schema with
  neither, such as an enum's own schema whose variants sit under `oneOf`,
  already admits a field that it does not describe.
- A name that a schema around the variant declares, such as a field of a
  struct that flattens the enum, or that a member of the schema's
  `oneOf`, `anyOf` or `allOf` declares, gains no null property, because
  that property would refuse the declared field's value.
- A variant that is a `$ref` is cut in its definition or in an inline
  copy, as ADR-0110 decided. The cut leaves the members of an `allOf`
  unchanged.
- A variant that declares the property keeps its schema. `--json --full`
  prints the full row and still matches the same schema, because a field
  that a variant lacks is absent from the full row.

## Consequences

`docs/behaviour/schemas/README.md` states the rule. A compact document of
an enum row whose variants differ in their fields matches the document
schema that crumbkit publishes, with or without
`#[schemars(deny_unknown_fields)]` on the row type.

Considered: leaving the field out of a row whose variant lacks it, which
changes the compact document's key set that ADR-0113 and ADR-0120 promise
and that readers rely on.
