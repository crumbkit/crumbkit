# ADR-0074. Options with a second use

- Date: 2026-10-17
- Status: amended by ADR-0122
- Supersedes: ADR-0017, which it amends
- Missed: ADR-0017 did not say what a record owes its reader when it amends a record from the same week, so a quick amendment could leave the reader to guess why the earlier decision fell short.

## Context

Before 0.1.0, crumbkit gained several `Document` options and a `Guide`
setting for one adopting tool. Each option reproduced bytes that the tool's
readers already parsed, such as a fixed type string, a listing without
`total` and summary fields at the top of a document. A review found that
these options encoded one tool's output, that no example used them, and that
a smaller API with glue in the tool would serve other tools better.

Several of those options arrived as records that amended a record from the
same week. ADR-0066 amended ADR-0059 two days after it, and ADR-0065 amended
ADR-0063 two days after it. Neither record said what the earlier one had
missed, so a reader could not tell whether the first decision was wrong or
whether a new need had appeared.

## Decision

- A new `Document` option, `Row` constant, `Guide` setting or `Leaf`
  setting needs a second adopter, or a use in `examples/` whose shape is
  not a copy of the first adopter's. The rule is API-1, and review enforces
  it, because no tool can tell a copied shape from an independent one.
- A new record whose `Supersedes` line names a record dated less than 7
  days before it carries a `- Missed:` line that says what the earlier
  record missed. The rule is DOCS-8. `cargo xtask check` and
  `cargo xtask check docs` fail on a record that is new on the branch, amends
  such a record and has no `- Missed:` line with text. The records already
  on `main` are frozen and are not checked.

## Consequences

An option that only one tool needs stays in that tool as glue until a second
use appears, so crumbkit's API grows more slowly and each option has an
example that shows it. A quick amendment costs one more line, and a reader of
the later record learns why the earlier one changed. The 7-day window counts
calendar days between the two `Date` lines, so an amendment 7 days or more
after its record needs no `Missed` line.

Considered: a rule for `Document` and `Row` options only, review alone for
the `Missed` line, a `## Missed` section in place of a line, a window of one
calendar week from Monday, and an xtask check that a second use exists.
