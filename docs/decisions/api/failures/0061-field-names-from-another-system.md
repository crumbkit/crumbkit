# ADR-0061. Field names from another system

- Date: 2026-10-14
- Status: superseded by ADR-0081
- Supersedes: ADR-0001 and ADR-0036, which it amends

## Context

A failure's field names followed the rule of a code segment: one or more
lowercase ASCII letters, digits and underscores. `finish` left out a field
that broke the rule, and the field-name rule had no opt-out because the
failure document's schema promised it to every reader. Some tools report
values whose names another system already chose, such as `requestId` from
an HTTP API or `Content-Type` from a header. Such a tool had to rename each
value before crumbkit printed it, so its users saw a different name in the
failure than in the system that the value came from.

## Decision

- A guide could list field names with `Guide::field_names`. `finish` and
  `stream` kept a field whose name the guide listed, though the name broke
  the rule, and still left out a field whose name repeated an earlier one.
- The schema that a tool served with `<tool> schema` for its failure
  document, and for each of its documents with a batch, accepted the listed
  names beside the rule in `fields`. A tool that listed no names served the
  schema with the rule alone. The checked-in `crumbkit/failure/v1` schema
  kept the rule alone, because it describes the document for every tool.
- The list was a declaration. `check_opt_outs` listed it once as the
  `field-names` customisation of every run, with no reason and no cost.
- `check_guide` reported a listed name that was empty, held whitespace or a
  control character, was listed twice, or already followed the rule.
- A tool with a failure form served its own schema, so the listed names did
  not change it.

## Consequences

A reader of a tool's served failure schema learns every field name that the
tool may print. A reader that holds only the checked-in schema may meet a
field name that the rule does not allow, so such a reader reads the served
schema of the tool that it calls. The name rule still applies to every
field name that the guide does not list. The guide holds the names as a
borrowed slice, so a run copies no names.

Considered: an opt-out with a reason for each field name, a rule wide
enough for every name that other systems use, a field name rule per leaf,
renaming each listed name to the rule on output, a checked-in schema that
accepts any name.
