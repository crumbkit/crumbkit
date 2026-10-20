# ADR-0122. Options for an ordinary need

- Date: 2026-10-20
- Status: accepted
- Supersedes: ADR-0074, which it amends
- Missed: ADR-0074 set the second-use bar against options that reproduced one tool's particular bytes, and it did not distinguish those from an ordinary need that one adopter happened to raise first. The bar therefore held back ordinary requests, such as a column left out when every row lacks it and child rows under a parent, until a second adopter appeared.

## Context

The owner reviewed an adopting tool's requests for new options. Some asked
crumbkit to print that tool's own bytes, and others asked for behaviour that
most tools with listings would want. The owner chose that one adopter with
an ordinary request is enough.

## Decision

- A new `Document` option, `Row` constant, `Guide` setting or `Leaf`
  setting has a second adopter, a use in `examples/` whose shape is not a
  copy of the first adopter's, or one adopter's request for an ordinary need
  that other tools are likely to share. The rule is API-1.
- An option that encodes one tool's particular output stays in that tool as
  glue.
- The pull request that adds the option names which of the three grounds it
  meets. Review enforces the rule, because no tool can tell a copied shape
  from an independent one or an ordinary need from a particular one.

## Consequences

An adopter's ordinary request can become an option without waiting for a
second adopter. Review judges whether a need is ordinary, and the pull
request states the ground so that a reader can check the judgement.

Considered: keeping the second-use bar, accepting any adopter's request, and
listing the ordinary needs in advance.
