# ADR-0073. One JSON style

- Date: 2026-10-17
- Status: accepted
- Supersedes: ADR-0052 and ADR-0067, which it amends
- Missed: ADR-0067 added a second guide setting for the JSON that a tool prints beside ADR-0052's, with a one-variant enum that no second order was planned for. ADR-0052 also used a one-variant enum for the layout.

## Context

A guide set the layout of its JSON with `Guide::json_layout(JsonLayout,
reason)` and the order of its keys with `Guide::json_keys(KeyOrder)`.
Both enums had one variant. crumbkit already combined the two settings
into one internal style value that every writer of JSON reads.

## Decision

- `Guide::json(JsonStyle)` sets the layout and the key order together, and
  `Guide::get_json` returns the style.
- `JsonStyle` is a public `#[non_exhaustive]` struct with two public
  fields: `layout: JsonLayout` and `sorted_keys: bool`. A tool builds it
  with the `const fn` constructors `JsonStyle::new`, `pretty(reason)` and
  `sorted`.
- `JsonLayout` has the variants `OneLine`, the default, and
  `Pretty { reason }`. The layout carries the reason because it is an
  opt-out, and sorted keys are a declaration without one.
- `KeyOrder` is removed. ADR-0067 already rejected other orders.
- This departs from ADR-0002's wording that a configuration type keeps its
  fields private. The struct is `#[non_exhaustive]`, so crumbkit can still
  add a field without breaking a tool, and the public fields let a tool and
  crumbkit-test read the style without getters.

## Consequences

A guide sets its JSON with one builder call, such as
`Guide::new().json(JsonStyle::new().pretty("...").sorted())`. Everything
else that ADR-0052 and ADR-0067 decided is unchanged.

Considered: a `reason` argument on `Guide::json`, which a style that only
sorts keys would have to pass; a two-variant `KeyOrder`; private fields
with getters.
