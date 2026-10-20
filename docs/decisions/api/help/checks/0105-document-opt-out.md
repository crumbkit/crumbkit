# ADR-0105. An opt-out for the document rule of check_guide

- Date: 2026-10-20
- Status: amended by ADR-0115
- Supersedes: ADR-0016 and ADR-0104, which it amends
- Missed: ADR-0104 added an opt-out for the example rule and missed one for the document rule, which also stops a tool that uses crumbkit's help before it uses crumbkit's documents.

## Context

`check_guide` fails a leaf whose effects change something and that
declares no document, because crumbkit then refuses a view flag that the
output cannot use only after the change (ADR-0016). ADR-0016 considered an
opt-out for a changing leaf whose output a tool cannot describe and
rejected it. An adopting tool prints its own output and uses crumbkit only
for help. It could not declare the effects of its leaves that change
something until it moved its output onto crumbkit's documents, because
each such leaf then failed the check.

## Decision

- The document rule has an opt-out, `Rule::GuideDocument`, whose id is
  `guide/document`. An opt-out from it covers the command it names and
  every command below it.
- `check_guide` fails on an opt-out from `guide/document` that names no
  command or excuses no problem.
- The rule's costs say that crumbkit cannot refuse a view flag that the
  output cannot use before the change happens, so the tool's own output
  decides what the flag does.

## Consequences

`docs/behaviour/help/examples.md` and `docs/behaviour/adoption/opt-outs.md`
describe the current behaviour. No example tool uses the opt-out, because
each of their leaves that changes something declares a document, so the
adopting tool is its first user.

Considered: leaving the rule without an opt-out, which keeps the tool from
declaring its effects; an opt-out that covers only the command it names.
