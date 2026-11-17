# ADR-0236. An unprovoked entry can list a whole area

- Date: 2026-11-17
- Status: accepted
- Amends: ADR-0053, ADR-0177

## Context

A tool whose failures forward codes from library crates declares every one
of them. Both checks then need a start, a twin or an `Unprovoked::new` entry
for each declared code. A tool with hundreds of codes that its binary never
prints, such as those of a server API, wrote one entry with the same reason
for each.

## Decision

- `Unprovoked::area(component, area, reason)` lists every declared code of
  one area under one reason. Both checks accept it wherever they accept
  `Unprovoked::new`.
- Both checks share one set of rules for the entries, in one function.
- An area entry that covers no declared code, an area entry whose every
  declared code a start provokes or a twin refuses, and a code entry inside
  a listed area are problems. An untwinned opt-out for a code in a listed
  area is stale.
- The recovery report prints an area entry as `tool::api::*: reason` and
  each declared code it excuses below it.
- The codes a binary can print are the code list that a tool passes to
  `Recoveries::new` and `Twins::new`. No separate setting holds them.

## Consequences

An area entry excuses a code added to the area later, and no check fails.
The recovery report's golden file shows the new code, so review sees it. An
area entry trusts its reason as a code entry does, and no check proves the
area holds no code a test could provoke.

Considered: an entry for one of the two checks only; a wildcard over a
whole component; reading the printed codes from the binary.
