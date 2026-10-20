# ADR-0115. The document rule of check_guide covers only a command that takes a view flag

- Date: 2026-10-20
- Status: accepted
- Supersedes: ADR-0105, which it amends
- Missed: ADR-0105 added the opt-out and missed that a leaf whose command takes no view flag has nothing for the rule to protect.

## Context

`check_guide` fails a changing leaf without a document, because crumbkit
refuses an unusable view flag only after the change, and an agent may
repeat the write (ADR-0016, ADR-0105). An adopting tool prints its own
output, and some of its commands take no view flag, so no such refusal
can happen. The rule still failed those leaves, so the tool left their
effects undeclared, because a read-only effect would be false.

## Decision

- The document rule applies only to a leaf whose command takes a view
  flag: the nearest command, from it up to the root, that declares an
  argument with the id `json`, `fields`, `long` or `full` declares it
  global, as `parse` reads one.
- An opt-out from `guide/document` on a leaf whose command takes no view
  flag excuses no problem, and the check fails on it.

## Consequences

`docs/behaviour/` describes the current rule.

Considered: keeping the rule with the opt-out as the only way out, which
makes a tool declare a false read-only effect or leave its effects
undeclared; a `Leaf` setting that declares that the leaf prints its own
output, which API-1 keeps out until a second use appears.
