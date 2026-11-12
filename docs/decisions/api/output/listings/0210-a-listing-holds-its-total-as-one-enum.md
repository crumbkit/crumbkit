# ADR-0210. A listing holds its total as one enum

- Date: 2026-11-12
- Status: accepted
- Supersedes: ADR-0087 and ADR-0130, which it amends

## Context

`Listing` held its total in three fields: `total: Option<usize>`, `more`,
which only a listing without a total read, and `capped`, which only a
listing with a total read. Several combinations of the fields contradicted
each other, and three bugs came from them, among them a JSON document that
held `"total": 5` beside `"truncated": true`. `Listing::total` returned
`Option<usize>`, so a caller could not tell a capped listing from an
uncounted one.

## Decision

- `crumbkit::Total` holds the total once:
  - `Exact(n)`: the tool counted every matching row, and `n` matched.
  - `AtLeast(n)`: the tool read `n` rows from a source that holds more.
  - `Unknown { more }`: the tool did not count, and `more` says whether
    the source holds rows after the listing's rows.
- `Listing` stores one `Total`. `Listing::new` gives `Exact`, raised to at
  least the number of rows, `Listing::uncounted` gives `Unknown`, and
  `Listing::capped(true)` turns `Exact` into `AtLeast` and leaves `Unknown`
  as it is. The constructors keep their signatures.
- `Listing::total` returns `Total`. `Total::exact` returns the count of
  `Exact` and `None` otherwise, which is the old answer.
- `Total` is not `#[non_exhaustive]`, so a caller's `match` names all three
  variants.
- The text footer, the cut line and the JSON document are unchanged.

## Consequences

A listing cannot hold a combination that contradicts itself. A tool that
read `Listing::total` as `Option<usize>` fails to build until it calls
`Total::exact`. `docs/behaviour/output/listings/README.md` describes the
current behaviour.

Considered: a plain `Unknown`, which loses whether an uncounted source holds
more rows; new constructor signatures, which the enum makes unnecessary;
`Total` as the return type of the sealed `Rows::total`, which a list and a
single row cannot fill.
